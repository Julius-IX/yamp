# cloud-auth

The sign in for Google Drive and OneDrive. `cloud-sync` moves the bytes and leaves the login to
the application, this crate is that login. It is the authorization code flow with PKCE, and the
way the browser finds its way back to the app is the one each platform is meant to use:

| Platform              | The answer comes back as                                                      |
| --------------------- | ----------------------------------------------------------------------------- |
| Linux, macOS, Windows | a request to a port the app listens on, `http://127.0.0.1:<port>`             |
| Android, iOS          | a link of a scheme of the app, which the system hands to it (deep link plugin) |

Because the platforms sign in differently, every platform gets a client of its own. The service
then sees which platform a sign in comes from, and a client can be revoked or limited on its own.

## Which services a build has

The id of every client is read from the environment when the app is compiled, and **a service with
no id for the platform being built does not exist in that build**. It is not shown in the
settings, and when no service is set up the Data section says that no cloud services are
available.

Every service has seven levels of variables, named `YAMP_<SERVICE>_<LEVEL>_CLIENT_ID` and, where
the service has one, `YAMP_<SERVICE>_<LEVEL>_CLIENT_SECRET`:

| Level     | Used by                                                      |
| --------- | ------------------------------------------------------------ |
| `DESKTOP` | Linux, macOS and Windows, unless their own level is set      |
| `LINUX`   | Linux builds, wins over `DESKTOP`                            |
| `MACOS`   | macOS builds, wins over `DESKTOP`                            |
| `WINDOWS` | Windows builds, wins over `DESKTOP`                          |
| `MOBILE`  | Android and iOS, unless their own level is set              |
| `ANDROID` | Android builds, wins over `MOBILE`                           |
| `IOS`     | iOS builds, wins over `MOBILE`                               |

A build looks at the level of its own system first and then at the generic one of its kind. An
empty value counts as not set. The levels of other platforms are never used, and neither is a
secret from a different level than the id.

`DESKTOP` is enough for most setups, the three desktop systems can share one client. `MOBILE` only
makes sense for Microsoft, where one registration can carry the redirect of both phones. Google
needs `ANDROID` and `IOS`, since they are different kinds of client.

Copy `config.toml.example` to `config.toml` in this folder and fill in the ids. The build script
of this crate reads it from there, so it does not matter which folder cargo, `tauri dev` or Android
Studio is started in. These are the places it looks, the first one with a value wins:

1. the environment of the build, which is what a CI build sets,
2. `config.toml` in this folder,
3. the `[env]` table of `src-tauri/.cargo/config.toml`.

An empty value counts as not set, so a blank line in one place does not hide a value in another.
Change a value and the next build picks it up.

**Do not commit `config.toml`.** The ids are not secrets, every build carries them. But a fork
that builds with them shares the quota of this client and shows up as this app on the consent
screen, and if it is abused the service can disable the client. A fork registers its own. The
`.gitignore` of this folder keeps the file out, and only the `.example` file is committed.

## When a service is missing

The settings only show a service the build has a client for, and the reason for one that is left
out is written to the log. Open **Settings > Debugging > Logs** and look for a line like

> Google Drive is not set up in this build, none of YAMP_GOOGLE_ANDROID_CLIENT_ID or
> YAMP_GOOGLE_MOBILE_CLIENT_ID has a value

It names the variables this build looked at, which are the ones of its own platform. A client
filled in for another platform does not count. If the name is right and the value is in a file, the
build did not see the file, check that it is the `config.toml` in this folder and rebuild.

## The scheme of the app

The mobile flows need the system to open the app for a link. The identifier of the app,
`com.salad.yamp`, is registered for that in `tauri.conf.json` under `plugins > deep-link`, and it is
all that Android needs. Microsoft sends the browser back to `com.salad.yamp://auth` and an Android
client of Google to `com.salad.yamp:/oauth2redirect`, since Google only accepts the package name as
the scheme of an Android client. Sending anything else, such as the client id turned around, gets the
request blocked before the sign in page. If the redirect is for a scheme the app did not register,
the browser has nothing to open and the settings keep saying "Waiting for the browser...".

An iOS client of Google is the exception, it wants the client id turned around as the scheme: for
the client id `123-abc.apps.googleusercontent.com` it is `com.googleusercontent.apps.123-abc`. It
depends on the client, so it is not in the committed config. Put it in
`src-tauri/tauri.ios.conf.json`, which Tauri merges over `tauri.conf.json` on iOS, and keep the file
out of git like `config.toml`. A merge replaces a list, so the app's own scheme has to be in it
again:

```json
{
  "plugins": {
    "deep-link": {
      "mobile": [
        {
          "scheme": ["com.salad.yamp", "com.googleusercontent.apps.123-abc"],
          "appLink": false
        }
      ]
    }
  }
}
```

The Android activity has to be `singleTask`, which the Tauri template is, or a link starts a second
copy of the app instead of reaching the one that waits.

## Google Drive

You need a Google account and nothing else, no billing is asked for.

1. Open <https://console.cloud.google.com> and create a project.
2. **APIs & Services > Library**, search for **Google Drive API** and enable it.
3. Open **Google Auth Platform**, it is in the menu of the console or found with the search bar.
   This is what used to be called the OAuth consent screen. If it says it is not configured, press
   **Get started** and fill in the app name, a support email, **External** as the audience and a
   contact email.
4. **Data Access > Add or remove scopes**, and add the scope
   `https://www.googleapis.com/auth/drive.appdata`. It gives access to a hidden folder that only
   this app can see, and it is not a sensitive scope, so there is no security assessment.
5. **Audience > Publish app**. While the app is in **Testing**, only the test users you list can
   sign in and every sign in ends after 7 days. Leaving the logo out of **Branding** keeps the
   publishing free of a branding review.
6. **Clients > Create client**, one per platform you build for:

| Level                              | Application type | Settings                                                                  |
| ---------------------------------- | ---------------- | ------------------------------------------------------------------------- |
| `DESKTOP`, `LINUX`, `MACOS`, `WINDOWS` | **Desktop app** | none. Google asks for the secret, it protects nothing                    |
| `ANDROID`                          | **Android**      | package name `com.salad.yamp`, the SHA-1 of the signing certificate, and under Advanced settings **Enable custom URI scheme**. Without that option Google blocks the sign in |
| `IOS`                              | **iOS**          | bundle id `com.salad.yamp`                                                |

Put the ids, and the secrets of the desktop ones, in `config.toml` under the level they are for.
The Android and iOS clients have no secret, Google does not make one for them, and the variable is
simply left out. The SHA-1 of a debug build and of a release build
differ, a client carries one, so register one per kind of build you test and ship.

**About Android.** Google's recommended way for an Android app to get access to Drive is the
authorization API of Google Play services. That is a different flow, it needs the Play services on
the phone and a small Android plugin, and it is not built here. This crate uses the Android client
with a custom scheme, which Google still supports when the option above is on, but calls not
recommended. If Google stops accepting it, Android needs that plugin.

## OneDrive

Microsoft only lets an app be registered inside a **Microsoft Entra tenant**, a personal Microsoft
account does not come with one. The ways to get one:

- A free Azure account at <https://azure.microsoft.com/free>. It asks for a phone number and a
  payment method to verify the person. The credit of the trial runs out after 30 days, but the
  tenant is on the free plan of Entra ID and registering apps in it costs nothing, also after the
  trial.
- The Microsoft 365 Developer Program, a free sandbox tenant. Microsoft limits who can join it.

With a tenant, open the **Microsoft Entra admin center**, then **App registrations > New
registration**. Do it once per level you want to set, or once for a few if they can share:

1. Supported account types: **Accounts in any organizational directory and personal Microsoft
   accounts**. OneDrive accounts of people cannot sign in otherwise.
2. **Authentication > Add a platform > Mobile and desktop applications**, and add the redirects of
   the platforms the registration is for. Use `http://localhost` for the desktop levels, the port is
   ignored. Use the custom redirect `com.salad.yamp://auth` for the mobile levels.
3. Allow public client flows.
4. **API permissions > Add a permission > Microsoft Graph > Delegated**, add
   `Files.ReadWrite.AppFolder` and `offline_access`.

Put the application (client) id in `config.toml` under the level it is for. There is no secret.
Skipping OneDrive is fine. Leave its ids empty and it is not offered.

## iCloud

iCloud has no sign in. It needs the ubiquity container of the app from native code, which the app
does not provide yet, so it is never offered.

## Costs and conditions

Registering the clients and using them costs nothing, and the data lives in the hidden app folder
of the account of the user, so it counts against their storage and not against anything of yours.
The conditions are the services' to change, so check their current terms. The account that owns a
registration has to stay in good standing, losing it or breaking a service's terms takes the sign
in down for everyone who uses that service. A Google token that goes unused for about 6 months
expires, which the automatic sync prevents.

## What is stored

The refresh token and the account name go to `cloud_tokens.json` in the app data folder, readable
by the owner only on Unix. Access tokens live in memory.
