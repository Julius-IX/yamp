use std::time::Duration;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

use crate::{Error, Result, app::OAuthApp};

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Deserialize)]
pub(crate) struct TokenResponse {
  pub access_token: String,
  pub refresh_token: Option<String>,
  pub expires_in: Option<u64>,
  pub id_token: Option<String>,
}

/// What a finished sign in hands over.
pub struct SignedIn {
  pub refresh_token: String,
  pub access_token: String,
  pub expires_in: u64,
  /// The mail address or name of the account, for showing which one is linked.
  pub account: Option<String>,
}

fn post(url: &str, form: &[(&str, &str)]) -> Result<TokenResponse> {
  match ureq::post(url).timeout(TIMEOUT).send_form(form) {
    Ok(resp) => resp.into_json().map_err(|e| Error::Protocol(e.to_string())),
    Err(ureq::Error::Status(code, resp)) => {
      let body: serde_json::Value = resp.into_json().unwrap_or_default();
      let error = body["error"].as_str().unwrap_or("");
      let why = body["error_description"].as_str().unwrap_or(error);
      // the service no longer accepts the refresh token, as after a revoked grant
      if error == "invalid_grant" {
        return Err(Error::Revoked);
      }
      Err(Error::Protocol(format!("{code} {why}")))
    }
    Err(ureq::Error::Transport(t)) => Err(Error::Transport(t.to_string())),
  }
}

/// Trades the code of a sign in for tokens.
pub(crate) fn exchange(app: &OAuthApp, code: &str, verifier: &str, redirect: &str) -> Result<SignedIn> {
  let mut form = vec![
    ("grant_type", "authorization_code"),
    ("code", code),
    ("redirect_uri", redirect),
    ("client_id", app.client_id),
    ("code_verifier", verifier),
  ];
  if let Some(secret) = app.client_secret {
    form.push(("client_secret", secret));
  }
  if app.scope_in_token {
    form.push(("scope", app.scope));
  }
  let r = post(app.token_url, &form)?;
  let refresh_token = r
    .refresh_token
    .ok_or_else(|| Error::Protocol("no refresh token was sent".into()))?;
  Ok(SignedIn {
    refresh_token,
    access_token: r.access_token,
    expires_in: r.expires_in.unwrap_or(3600),
    account: r.id_token.as_deref().and_then(account_of),
  })
}

pub(crate) fn refresh(app: &OAuthApp, refresh_token: &str) -> Result<TokenResponse> {
  let mut form = vec![
    ("grant_type", "refresh_token"),
    ("refresh_token", refresh_token),
    ("client_id", app.client_id),
  ];
  if let Some(secret) = app.client_secret {
    form.push(("client_secret", secret));
  }
  if app.scope_in_token {
    form.push(("scope", app.scope));
  }
  post(app.token_url, &form)
}

/// The id token came straight from the service over TLS and is only read for a label, so its
/// signature is not checked.
fn account_of(id_token: &str) -> Option<String> {
  let payload = id_token.split('.').nth(1)?;
  let claims: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()?;
  ["email", "preferred_username", "name"]
    .iter()
    .find_map(|k| claims.get(k)?.as_str().map(str::to_owned))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn account_is_read_from_the_claims() {
    let claims = URL_SAFE_NO_PAD.encode(br#"{"email":"a@b.c","name":"A"}"#);
    assert_eq!(account_of(&format!("x.{claims}.y")).as_deref(), Some("a@b.c"));
    assert_eq!(account_of("garbage"), None);
  }
}
