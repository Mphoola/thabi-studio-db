//! Exporting and importing connection profiles as a JSON file.
//!
//! The file never holds a secret: passwords and passphrases stay in the
//! credential store, and an imported connection asks for them (it's saved
//! with nothing to send, so the first connect prompts or fails clearly).
//! Key and certificate files travel as paths only, as they are stored.

use super::{ConnectionProfile, ProfileInput, create_profile, list_profiles};
use crate::{credentials::CredentialStore, errors::AppError};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Identifies the file so an unrelated JSON file isn't half-imported.
const FORMAT: &str = "thabi-studio-connections";
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportFile {
    format: String,
    version: u32,
    connections: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported: usize,
    /// Connections that couldn't be imported, with the reason.
    pub skipped: Vec<SkippedConnection>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedConnection {
    pub name: String,
    pub reason: String,
}

/// The fields an export keeps: everything that describes the connection,
/// nothing that identifies this install (ids, dates) or holds a secret.
fn exported(profile: &ConnectionProfile) -> serde_json::Value {
    serde_json::json!({
        "name": profile.name,
        "environment": profile.environment,
        "driver": profile.driver,
        "host": profile.host,
        "port": profile.port.to_string(),
        "databaseName": profile.database_name,
        "username": profile.username,
        "tlsMode": profile.tls_mode,
        "tlsCaPath": profile.tls_ca_path,
        "tlsClientCert": profile.tls_client_cert,
        "tlsClientKey": profile.tls_client_key,
        "sshEnabled": profile.ssh_enabled,
        "sshHost": profile.ssh_host,
        "sshPort": profile.ssh_port.to_string(),
        "sshUsername": profile.ssh_username,
        "sshAuthMode": profile.ssh_auth_mode,
        "sshKeyPath": profile.ssh_key_path,
        "safeMode": profile.safe_mode,
        "groupName": profile.group_name,
        "colour": profile.colour,
        "askDatabasePassword": profile.ask_database_password,
        "askSshPassword": profile.ask_ssh_password,
        "askSshKeyPassphrase": profile.ask_ssh_key_passphrase,
        "sshKeepaliveSeconds": profile.ssh_keepalive_seconds.to_string(),
        "startupCommands": profile.startup_commands,
        "queryTimeoutSeconds": profile.query_timeout_seconds.to_string(),
        "maxResultRows": profile.max_result_rows.to_string(),
    })
}

/// Every saved connection as export JSON text.
pub fn export_json(connection: &Connection) -> Result<String, AppError> {
    let file = ExportFile {
        format: FORMAT.to_owned(),
        version: VERSION,
        connections: list_profiles(connection)?.iter().map(exported).collect(),
    };
    serde_json::to_string_pretty(&file).map_err(|_| AppError::storage())
}

/// Imports every connection in `text`. Each is validated as if typed into
/// the form; one that fails is skipped and named, the rest still import.
pub fn import_json(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    text: &str,
) -> Result<ImportSummary, AppError> {
    let not_ours = || {
        AppError::new(
            "invalid_import",
            "This file isn't a Thabi Studio connections export.",
        )
    };
    let file: ExportFile = serde_json::from_str(text).map_err(|_| not_ours())?;
    if file.format != FORMAT {
        return Err(not_ours());
    }
    if file.version > VERSION {
        return Err(AppError::new(
            "invalid_import",
            "This export was made by a newer Thabi Studio. Update the app to import it.",
        ));
    }
    let existing: Vec<String> = list_profiles(connection)?
        .into_iter()
        .map(|p| p.name)
        .collect();
    let mut summary = ImportSummary {
        imported: 0,
        skipped: Vec::new(),
    };
    for entry in file.connections {
        let name = entry
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Unnamed")
            .to_owned();
        let mut input: ProfileInput = match serde_json::from_value(entry) {
            Ok(input) => input,
            Err(_) => {
                summary.skipped.push(SkippedConnection {
                    name,
                    reason: "Some of its settings are missing or malformed.".into(),
                });
                continue;
            }
        };
        // Never trust a file to carry secrets in.
        input.database_password = None;
        input.ssh_password = None;
        input.ssh_key_passphrase = None;
        if existing.contains(&input.name) {
            input.name = format!("{} (imported)", input.name);
        }
        match create_profile(connection, credentials, input) {
            Ok(_) => summary.imported += 1,
            Err(error) => summary.skipped.push(SkippedConnection {
                name,
                reason: match error.fields {
                    Some(fields) => fields.values().cloned().collect::<Vec<_>>().join(" "),
                    None => error.message,
                },
            }),
        }
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::validation::tests::input;
    use crate::storage::migrate;
    use std::{collections::HashMap, sync::Mutex};

    #[derive(Default)]
    struct Secrets(Mutex<HashMap<String, String>>);

    impl CredentialStore for Secrets {
        fn set(&self, reference: &str, value: &str) -> Result<(), AppError> {
            self.0
                .lock()
                .unwrap()
                .insert(reference.into(), value.into());
            Ok(())
        }
        fn get(&self, reference: &str) -> Result<Option<String>, AppError> {
            Ok(self.0.lock().unwrap().get(reference).cloned())
        }
        fn delete(&self, reference: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().remove(reference);
            Ok(())
        }
    }

    fn db() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
    }

    #[test]
    fn round_trips_everything_but_secrets() {
        let (source, secrets) = (db(), Secrets::default());
        let mut profile = input();
        profile.database_password = Some("hunter2-secret".into());
        profile.startup_commands = "SET search_path = app".into();
        create_profile(&source, &secrets, profile).unwrap();

        let text = export_json(&source).unwrap();
        assert!(!text.contains("hunter2-secret"), "no secret in the file");
        assert!(text.contains("SET search_path = app"));

        let (target, fresh) = (db(), Secrets::default());
        let summary = import_json(&target, &fresh, &text).unwrap();
        assert_eq!(summary.imported, 1);
        assert!(summary.skipped.is_empty());
        let imported = &list_profiles(&target).unwrap()[0];
        let original = &list_profiles(&source).unwrap()[0];
        assert_eq!(imported.name, original.name);
        assert_eq!(imported.host, original.host);
        assert_eq!(imported.startup_commands, original.startup_commands);
        assert!(!imported.has_database_password, "secrets are re-entered");
        assert!(fresh.0.lock().unwrap().is_empty(), "nothing stored");

        // Importing again keeps both, under a different name.
        import_json(&target, &fresh, &text).unwrap();
        let names: Vec<_> = list_profiles(&target)
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert!(names.iter().any(|n| n.ends_with("(imported)")), "{names:?}");
    }

    #[test]
    fn refuses_other_files_and_skips_bad_entries() {
        let (connection, secrets) = (db(), Secrets::default());
        assert_eq!(
            import_json(&connection, &secrets, "{\"rows\":[]}")
                .unwrap_err()
                .kind,
            "invalid_import"
        );
        let file = serde_json::json!({
            "format": FORMAT,
            "version": 1,
            "connections": [{ "name": "Broken", "port": 5 }],
        });
        let summary = import_json(&connection, &secrets, &file.to_string()).unwrap();
        assert_eq!(summary.imported, 0);
        assert_eq!(summary.skipped[0].name, "Broken");
    }
}
