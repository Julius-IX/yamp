use cloud_sync::Provider;

use crate::{Error, Result};

/// What differs between the services in the flow, the rest is shared.
pub(crate) struct OAuthApp {
  pub auth_url: &'static str,
  pub token_url: &'static str,
  pub scope: &'static str,
  pub client_id: &'static str,
  /// Google asks for it even from apps that cannot keep it secret, so it is no real secret here.
  pub client_secret: Option<&'static str>,
  /// Where the service sends the browser once the user has answered.
  pub redirect: Redirect,
  /// Added to the authorization request.
  pub extra: &'static [(&'static str, &'static str)],
  /// Microsoft wants the scope repeated in the token request.
  pub scope_in_token: bool,
}

/// How the answer of a sign in gets back to the app. It is what differs most between platforms.
#[derive(Debug, PartialEq)]
pub(crate) enum Redirect {
  /// The browser goes to a port the app listens on. This is how desktop apps are meant to do it.
  /// Google registers the address and Microsoft only accepts the name, hence the host.
  Loopback { host: &'static str },
  /// The browser opens a link of a scheme that belongs to the app, which the operating system
  /// hands to it. This is how the mobile apps are meant to do it.
  Scheme(String),
}

/// The kind of system being built. The platforms differ in how a service wants the answer sent
/// back, so the redirect is chosen from this.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Platform {
  Desktop,
  Android,
  Ios,
}

impl Platform {
  /// A phone has no use for a listener on a local port, it gets the answer as a link instead.
  const fn current() -> Self {
    if cfg!(target_os = "android") {
      Self::Android
    } else if cfg!(target_os = "ios") {
      Self::Ios
    } else {
      Self::Desktop
    }
  }
}

/// `app_id` is the identifier of the app, which is also the package name on Android and the scheme
/// Microsoft sends the browser to.
pub(crate) fn redirect_for(provider: Provider, client_id: &str, app_id: &str, platform: Platform) -> Redirect {
  match (provider, platform) {
    (Provider::GoogleDrive, Platform::Desktop) => Redirect::Loopback { host: "127.0.0.1" },
    // An Android client only accepts the package name as the scheme. The client id turned around is
    // the scheme of an iOS client, and sending it for an Android one gets the request blocked.
    (Provider::GoogleDrive, Platform::Android) => Redirect::Scheme(format!("{app_id}:/oauth2redirect")),
    (Provider::GoogleDrive, Platform::Ios) => Redirect::Scheme(format!(
      "com.googleusercontent.apps.{}:/oauth2redirect",
      client_id.strip_suffix(".apps.googleusercontent.com").unwrap_or(client_id)
    )),
    (Provider::OneDrive, Platform::Desktop) => Redirect::Loopback { host: "localhost" },
    (Provider::OneDrive, _) => Redirect::Scheme(format!("{app_id}://auth")),
    // has no sign in, so no answer ever comes back
    #[cfg(target_vendor = "apple")]
    (Provider::ICloud, _) => Redirect::Loopback { host: "127.0.0.1" },
  }
}

/// An unset or empty variable counts as not set.
#[cfg(test)]
fn env(value: Option<&'static str>) -> Option<&'static str> {
  value.filter(|v| !v.is_empty())
}

/// One level of the config: the variable of the id, the id and the secret.
type Level = (&'static str, Option<&'static str>, Option<&'static str>);

/// The variables are read when the app is built. Level names are the middle part of
/// `YAMP_<SERVICE>_<LEVEL>_CLIENT_ID` and `..._CLIENT_SECRET`.
macro_rules! level {
  ($service:literal, $level:literal) => {
    (
      concat!("YAMP_", $service, "_", $level, "_CLIENT_ID"),
      option_env!(concat!("YAMP_", $service, "_", $level, "_CLIENT_ID")),
      option_env!(concat!("YAMP_", $service, "_", $level, "_CLIENT_SECRET")),
    )
  };
}

/// The levels that apply to the platform being built, most specific first. Every OS has a level
/// of its own that wins over the generic one of its kind, desktop for Linux, macOS and Windows and
/// mobile for Android and iOS. A level of another platform is never looked at.
macro_rules! levels {
  ($service:literal) => {{
    #[cfg(target_os = "linux")]
    let levels = vec![level!($service, "LINUX"), level!($service, "DESKTOP")];
    #[cfg(target_os = "macos")]
    let levels = vec![level!($service, "MACOS"), level!($service, "DESKTOP")];
    #[cfg(target_os = "windows")]
    let levels = vec![level!($service, "WINDOWS"), level!($service, "DESKTOP")];
    #[cfg(target_os = "android")]
    let levels = vec![level!($service, "ANDROID"), level!($service, "MOBILE")];
    #[cfg(target_os = "ios")]
    let levels = vec![level!($service, "IOS"), level!($service, "MOBILE")];
    // any other system is treated as a desktop
    #[cfg(not(any(
      target_os = "linux",
      target_os = "macos",
      target_os = "windows",
      target_os = "android",
      target_os = "ios"
    )))]
    let levels = vec![level!($service, "DESKTOP")];
    levels
  }};
}

/// The first level with an id wins, and its secret goes with it. An empty id counts as not set,
/// so a service left empty everywhere does not exist in the build. The error names the variables
/// that were looked at, so a build that lost its config can be told from one that never had it.
fn pick(service: &'static str, levels: Vec<Level>) -> Result<(&'static str, Option<&'static str>)> {
  let vars = levels.iter().map(|l| l.0).collect::<Vec<_>>().join(" or ");
  levels
    .into_iter()
    .find_map(|(_, id, secret)| id.filter(|v| !v.is_empty()).map(|id| (id, secret.filter(|v| !v.is_empty()))))
    .ok_or(Error::NotConfigured { service, vars })
}

pub(crate) fn app(provider: Provider, app_id: &str) -> Result<OAuthApp> {
  match provider {
    Provider::GoogleDrive => {
      let (client_id, client_secret) = pick("Google Drive", levels!("GOOGLE"))?;
      Ok(OAuthApp {
        auth_url: "https://accounts.google.com/o/oauth2/v2/auth",
        token_url: "https://oauth2.googleapis.com/token",
        // drive.appdata is the hidden folder only this app can see, email is for the label
        scope: "https://www.googleapis.com/auth/drive.appdata openid email",
        client_id,
        client_secret,
        redirect: redirect_for(provider, client_id, app_id, Platform::current()),
        // offline asks for a refresh token, consent makes Google send it on every sign in
        extra: &[("access_type", "offline"), ("prompt", "consent")],
        scope_in_token: false,
      })
    }
    Provider::OneDrive => {
      let (client_id, _) = pick("OneDrive", levels!("MICROSOFT"))?;
      Ok(OAuthApp {
        auth_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
        token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        scope: "Files.ReadWrite.AppFolder offline_access openid profile email",
        client_id,
        client_secret: None,
        redirect: redirect_for(provider, client_id, app_id, Platform::current()),
        extra: &[("prompt", "select_account")],
        scope_in_token: true,
      })
    }
    #[cfg(target_vendor = "apple")]
    Provider::ICloud => Err(Error::NoSignIn("iCloud")),
  }
}

/// Whether a sign in can be started for the service in this build. The error says which
/// variables were looked at.
pub fn check(provider: Provider) -> Result<()> {
  app(provider, "").map(|_| ())
}

pub fn configured(provider: Provider) -> bool {
  check(provider).is_ok()
}

pub fn display_name(provider: Provider) -> &'static str {
  match provider {
    Provider::GoogleDrive => "Google Drive",
    Provider::OneDrive => "OneDrive",
    #[cfg(target_vendor = "apple")]
    Provider::ICloud => "iCloud",
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_most_specific_level_wins_with_its_own_secret() {
    let found = pick("S", vec![("OS", Some("os"), Some("secret")), ("GENERIC", Some("generic"), Some("other"))]);
    assert_eq!(found.unwrap(), ("os", Some("secret")));
  }

  #[test]
  fn the_generic_level_is_the_fallback_and_empty_counts_as_unset() {
    let found = pick("S", vec![("OS", Some(""), Some("secret")), ("GENERIC", Some("generic"), None)]);
    assert_eq!(found.unwrap(), ("generic", None));
  }

  #[test]
  fn desktop_listens_and_phones_are_sent_a_link() {
    let google = "123-abc.apps.googleusercontent.com";
    assert_eq!(
      redirect_for(Provider::GoogleDrive, google, "com.x.y", Platform::Desktop),
      Redirect::Loopback { host: "127.0.0.1" }
    );
    assert_eq!(
      redirect_for(Provider::OneDrive, "id", "com.x.y", Platform::Desktop),
      Redirect::Loopback { host: "localhost" }
    );
    assert_eq!(
      redirect_for(Provider::OneDrive, "id", "com.x.y", Platform::Android),
      Redirect::Scheme("com.x.y://auth".into())
    );
  }

  #[test]
  fn google_on_android_uses_the_package_name_and_on_ios_the_client_id() {
    let google = "123-abc.apps.googleusercontent.com";
    assert_eq!(
      redirect_for(Provider::GoogleDrive, google, "com.x.y", Platform::Android),
      Redirect::Scheme("com.x.y:/oauth2redirect".into())
    );
    assert_eq!(
      redirect_for(Provider::GoogleDrive, google, "com.x.y", Platform::Ios),
      Redirect::Scheme("com.googleusercontent.apps.123-abc:/oauth2redirect".into())
    );
  }

  #[test]
  fn nothing_set_names_every_variable_that_was_looked_at() {
    let err = pick("S", vec![("OS", None, None), ("GENERIC", None, None)]).unwrap_err();
    assert!(matches!(err, Error::NotConfigured { vars, .. } if vars == "OS or GENERIC"));
  }

  // runs with something to check only when the build was given ids, as a build of the app does
  #[cfg(target_os = "linux")]
  #[test]
  fn a_build_reads_the_levels_of_its_own_platform() {
    let expected = env(option_env!("YAMP_GOOGLE_LINUX_CLIENT_ID")).or(env(option_env!("YAMP_GOOGLE_DESKTOP_CLIENT_ID")));
    assert_eq!(app(Provider::GoogleDrive, "").ok().map(|a| a.client_id), expected);
  }
}
