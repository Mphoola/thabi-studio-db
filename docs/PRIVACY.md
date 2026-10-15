# Privacy

Thabi Studio is a local-first desktop app. It has **no user accounts, no
telemetry, no analytics and no cloud backend**. The only network connections
it makes are to the database servers and SSH servers you configure.

## What is stored, and where

| Data                                                                                             | Where                                                                                                                   | Notes                                                                                                 |
| ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Database passwords, SSH passwords, SSH key passphrases                                           | The OS credential store: Keychain (macOS), Windows Credential Manager, Secret Service / GNOME Keyring / KWallet (Linux) | Never written to the app's database, logs or error messages                                           |
| Connection settings (host, port, user, database, TLS and SSH options, SSH key file _path_)       | Local SQLite database in the app data folder                                                                            | Key files are read when connecting and never copied                                                   |
| Trusted SSH host keys                                                                            | Local SQLite database                                                                                                   | Your `~/.ssh/known_hosts` is read but never changed                                                   |
| Query history (SQL text, connection, database, time, duration, status, row count, error summary) | Local SQLite database                                                                                                   | Can be turned off, limited by age (default 30 days, capped at 10,000 queries) and cleared in Settings |
| Saved queries                                                                                    | Local SQLite database                                                                                                   | Until you delete them                                                                                 |
| Open query tabs (title, SQL, connection, database)                                               | Local SQLite database                                                                                                   | So unsaved work survives quitting; removed when the tab is closed                                     |
| Preferences                                                                                      | Local SQLite database                                                                                                   | Theme, layout, timeouts and similar                                                                   |
| Query results                                                                                    | Memory only                                                                                                             | Up to "Rows kept per result" rows per result, last 8 results per tab; gone when the tab or app closes |
| Exported CSV files                                                                               | Wherever you save them                                                                                                  |                                                                                                       |

The app data folder is shown in **About Thabi Studio**. Typical locations:

- macOS: `~/Library/Application Support/com.thabi.studio/`
- Windows: `%APPDATA%\com.thabi.studio\`
- Linux: `~/.local/share/com.thabi.studio/`

## Things to know

- Query history stores the SQL exactly as you ran it, including any values
  typed into it (for example, a password in a `CREATE USER` statement). Turn
  history off in Settings for sensitive work, or clear it afterwards.
- Error messages from database servers are shown and stored in history after
  any password is removed from them.
- Deleting a connection also deletes its saved passwords from the credential
  store.
- Uninstalling the app does not remove the app data folder or credential
  store entries; delete the folder and the `com.thabi.studio` entries to
  remove everything.
