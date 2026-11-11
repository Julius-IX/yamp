# YAMP Privacy Policy

**Effective date:** November 11, 2026

YAMP ("the app") is an open source application available at
<https://github.com/Julius-IX/yamp>. This policy explains what data the app handles, where it
goes, and how you can remove it.

**In short:** YAMP has no server and no developer-run backend. Nothing is sent to the developer.
Your data stays on your device, and goes to a cloud service only if you choose to connect one.

## Data stored on your device

YAMP stores its data in the app data folder of your device. This data stays there unless you
turn on cloud sync, described below. The developer cannot see it.

## Optional cloud sync (Google Drive and OneDrive)

Cloud sync is off until you sign in to a service yourself. If you do, the following applies.

### Google Drive

- **What the app accesses:** YAMP requests only the scope
  `https://www.googleapis.com/auth/drive.appdata`. This gives access to a hidden folder
  in your Google Drive that only YAMP can see. YAMP cannot see, read, or change any of your other
  Drive files.
- **What is stored there:** the app's own data that you choose to sync.
- **What is stored on your device:** a refresh token and your account name, in the file
  `cloud_tokens.json` in the app data folder. Access tokens are kept in memory only and are
  discarded when the app closes.

### OneDrive

- **What the app accesses:** YAMP requests the Microsoft Graph permissions
  `Files.ReadWrite.AppFolder` and `offline_access`. This gives access to a dedicated app folder
  in your OneDrive and does not give access to your other files.
- **What is stored on your device:** the same as for Google Drive: a refresh token and your
  account name in `cloud_tokens.json`.

## Data collection and sharing

- The developer does **not** collect, receive, or store any of your data, including your Google
  or Microsoft account data.
- YAMP does **not** sell, share, rent, or transfer your data to any third party.
- Data from your Google or Microsoft account is sent only to that service's own APIs, to read
  and write the app's folder, and to nowhere else.
- YAMP does not include advertising or analytics, and does not track you.
- The in-app log (Settings > Debugging > Logs) stays on your device.

## Google API Services User Data Policy

YAMP's use and transfer of information received from Google APIs adheres to the
[Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy),
including the Limited Use requirements.

## Retention and deletion

You can remove your data at any time:

1. **Sign out** of the service in YAMP's settings, or delete `cloud_tokens.json` from the app
   data folder, to remove the stored tokens from your device.
2. **Revoke access** for YAMP in your account:
   - Google: <https://myaccount.google.com/permissions>
   - Microsoft: <https://account.live.com/consent/Manage>
3. **Delete the synced data** by removing YAMP's app folder from your Google Drive or OneDrive,
   or by uninstalling the app and deleting its app data folder.

Data in your cloud account is stored under your own account and counts against your own
storage. The developer has no access to it.

## Children

YAMP is not directed at children and does not knowingly collect any personal information.

## Changes to this policy

If this policy changes, the updated version will be posted at this location with a new
effective date.

## Contact

For questions about this policy, contact: **Julius.Schutte@gmail.com**
