use std::{
  sync::{Mutex, PoisonError},
  time::{Duration, Instant},
};

use cloud_sync::{Provider, TokenProvider};

use crate::{
  Error, Result, SignedIn,
  app::{OAuthApp, app},
  store::{Saved, TokenStore},
  token::refresh,
};

/// Access tokens are renewed this long before they run out, so a request never carries one that
/// expires on the way.
const MARGIN: Duration = Duration::from_secs(60);

/// The tokens of one signed in account. It is what `cloud-sync` asks for a token.
pub struct Session {
  app: OAuthApp,
  provider: Provider,
  store: TokenStore,
  state: Mutex<State>,
}

struct State {
  refresh_token: String,
  account: Option<String>,
  access: Option<(String, Instant)>,
}

impl Session {
  /// Remembers a sign in that just finished.
  pub fn from_sign_in(provider: Provider, store: TokenStore, signed: SignedIn) -> Result<Self> {
    store.save(
      provider,
      Saved { refresh_token: signed.refresh_token.clone(), account: signed.account.clone() },
    )?;
    let until = Instant::now() + Duration::from_secs(signed.expires_in).saturating_sub(MARGIN);
    Ok(Self {
      // the redirect is only needed to start a sign in
      app: app(provider, "")?,
      provider,
      store,
      state: Mutex::new(State {
        refresh_token: signed.refresh_token,
        account: signed.account,
        access: Some((signed.access_token, until)),
      }),
    })
  }

  /// Picks up the sign in of an earlier run. No request is made until a token is needed.
  pub fn from_store(provider: Provider, store: TokenStore) -> Result<Self> {
    let saved = store.load(provider).ok_or(Error::NotSignedIn)?;
    Ok(Self {
      // the redirect is only needed to start a sign in
      app: app(provider, "")?,
      provider,
      store,
      state: Mutex::new(State { refresh_token: saved.refresh_token, account: saved.account, access: None }),
    })
  }

  pub fn account(&self) -> Option<String> {
    self.state.lock().unwrap_or_else(PoisonError::into_inner).account.clone()
  }

  fn renew(&self, state: &mut State) -> Result<String> {
    let r = refresh(&self.app, &state.refresh_token)?;
    // Microsoft hands out a new refresh token with every renewal and drops the old one
    if let Some(next) = r.refresh_token.filter(|t| *t != state.refresh_token) {
      state.refresh_token = next;
      self.store.save(
        self.provider,
        Saved { refresh_token: state.refresh_token.clone(), account: state.account.clone() },
      )?;
    }
    let until = Instant::now() + Duration::from_secs(r.expires_in.unwrap_or(3600)).saturating_sub(MARGIN);
    state.access = Some((r.access_token.clone(), until));
    Ok(r.access_token)
  }
}

impl TokenProvider for Session {
  fn token(&self, force_refresh: bool) -> cloud_sync::Result<String> {
    let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
    if !force_refresh {
      if let Some((token, until)) = &state.access {
        if Instant::now() < *until {
          return Ok(token.clone());
        }
      }
    }
    self.renew(&mut state).map_err(|e| match e {
      Error::Revoked | Error::NotSignedIn => cloud_sync::Error::Unauthorized,
      Error::Transport(why) => cloud_sync::Error::Transport(why),
      other => cloud_sync::Error::Protocol(other.to_string()),
    })
  }
}
