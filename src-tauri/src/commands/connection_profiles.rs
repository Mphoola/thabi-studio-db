use crate::{credentials::CredentialStore, errors::AppError, state::AppState};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tauri::State;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub environment: String,
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
    pub username: String,
    pub auth_mode: String,
    pub tls_mode: String,
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub ssh_auth_mode: String,
    pub has_database_password: bool,
    pub has_ssh_password: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    pub environment: String,
    pub driver: String,
    pub host: String,
    pub port: String,
    pub database_name: String,
    pub username: String,
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_username: String,
    #[serde(default)]
    pub database_password: Option<String>,
    #[serde(default)]
    pub ssh_password: Option<String>,
    #[serde(default)]
    pub clear_database_password: bool,
    #[serde(default)]
    pub clear_ssh_password: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileWriteResult {
    pub profile: ConnectionProfile,
    pub warning: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CredentialBundle {
    database_password: Option<String>,
    ssh_password: Option<String>,
}

impl CredentialBundle {
    fn is_empty(&self) -> bool {
        self.database_password.is_none() && self.ssh_password.is_none()
    }
}

struct ValidatedProfile {
    name: String,
    environment: String,
    driver: String,
    host: String,
    port: u16,
    database_name: String,
    username: String,
    ssh_enabled: bool,
    ssh_host: String,
    ssh_port: u16,
    ssh_username: String,
    database_password: Option<String>,
    ssh_password: Option<String>,
    clear_database_password: bool,
    clear_ssh_password: bool,
}

fn profile_from_row(row: &Row<'_>) -> rusqlite::Result<ConnectionProfile> {
    Ok(ConnectionProfile {
        id: row.get("id")?,
        name: row.get("name")?,
        environment: row.get("environment")?,
        driver: row.get("driver")?,
        host: row.get("host")?,
        port: u16::try_from(row.get::<_, i64>("port")?)
            .map_err(|_| rusqlite::Error::IntegralValueOutOfRange(5, 0))?,
        database_name: row.get("database_name")?,
        username: row.get("username")?,
        auth_mode: row.get("auth_mode")?,
        tls_mode: row.get("tls_mode")?,
        ssh_enabled: row.get::<_, i64>("ssh_enabled")? != 0,
        ssh_host: row.get("ssh_host")?,
        ssh_port: row.get("ssh_port")?,
        ssh_username: row.get("ssh_username")?,
        ssh_auth_mode: row.get("ssh_auth_mode")?,
        has_database_password: row.get::<_, i64>("has_database_password")? != 0,
        has_ssh_password: row.get::<_, i64>("has_ssh_password")? != 0,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn profile_columns() -> &'static str {
    "id, name, environment, driver, host, port, database_name, username,
     auth_mode, tls_mode, ssh_enabled, ssh_host, ssh_port, ssh_username,
     ssh_auth_mode, has_database_password, has_ssh_password, created_at, updated_at"
}

fn get_profile(connection: &Connection, id: &str) -> Result<ConnectionProfile, AppError> {
    connection
        .query_row(
            &format!(
                "SELECT {} FROM connection_profiles WHERE id = ?1",
                profile_columns()
            ),
            [id],
            profile_from_row,
        )
        .optional()
        .map_err(|_| AppError::storage())?
        .ok_or_else(|| AppError::new("not_found", "That saved connection couldn't be found."))
}

fn get_credential_key(connection: &Connection, id: &str) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT credential_key FROM connection_profiles WHERE id = ?1",
            [id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| AppError::storage())?
        .ok_or_else(|| AppError::new("not_found", "That saved connection couldn't be found."))
}

fn validate(input: ProfileInput) -> Result<ValidatedProfile, AppError> {
    let mut fields = BTreeMap::new();

    let name = input.name.trim().to_owned();
    if name.is_empty() {
        fields.insert("name".into(), "Enter a connection name.".into());
    } else if name.chars().count() > 80 {
        fields.insert("name".into(), "Use 80 characters or fewer.".into());
    }

    if !matches!(
        input.environment.as_str(),
        "local" | "development" | "testing" | "staging" | "production"
    ) {
        fields.insert("environment".into(), "Choose an environment.".into());
    }
    if !matches!(input.driver.as_str(), "mysql" | "postgres" | "sqlserver") {
        fields.insert("driver".into(), "Choose a supported database.".into());
    }

    let host = input.host.trim().to_owned();
    if !valid_host(&host) {
        fields.insert(
            "host".into(),
            "Enter a valid host name or IP address.".into(),
        );
    }
    let port = validate_port(&input.port, "port", &mut fields);

    let database_name = input.database_name.trim().to_owned();
    if database_name.chars().count() > 128 || database_name.chars().any(char::is_control) {
        fields.insert("databaseName".into(), "Use a valid database name.".into());
    }

    let username = input.username.trim().to_owned();
    if username.is_empty() {
        fields.insert("username".into(), "Enter a user name.".into());
    } else if username.chars().count() > 128 || username.chars().any(char::is_control) {
        fields.insert("username".into(), "Use 128 characters or fewer.".into());
    }

    let ssh_host = input.ssh_host.trim().to_owned();
    let ssh_username = input.ssh_username.trim().to_owned();
    let ssh_port = if input.ssh_enabled {
        if !valid_host(&ssh_host) {
            fields.insert(
                "sshHost".into(),
                "Enter a valid SSH host name or IP address.".into(),
            );
        }
        if ssh_username.is_empty() {
            fields.insert("sshUsername".into(), "Enter an SSH user name.".into());
        } else if ssh_username.chars().count() > 128 || ssh_username.chars().any(char::is_control) {
            fields.insert("sshUsername".into(), "Use 128 characters or fewer.".into());
        }
        validate_port(&input.ssh_port, "sshPort", &mut fields)
    } else {
        22
    };

    for (field, secret) in [
        ("databasePassword", input.database_password.as_ref()),
        ("sshPassword", input.ssh_password.as_ref()),
    ] {
        if secret.is_some_and(|value| value.chars().count() > 4096) {
            fields.insert(field.into(), "Use 4,096 characters or fewer.".into());
        }
    }
    if input.clear_database_password
        && input
            .database_password
            .as_deref()
            .is_some_and(|s| !s.is_empty())
    {
        fields.insert(
            "databasePassword".into(),
            "Enter a password or choose to remove the saved password, not both.".into(),
        );
    }
    if input.clear_ssh_password && input.ssh_password.as_deref().is_some_and(|s| !s.is_empty()) {
        fields.insert(
            "sshPassword".into(),
            "Enter a password or choose to remove the saved password, not both.".into(),
        );
    }

    if !fields.is_empty() {
        return Err(AppError::validation(fields));
    }

    Ok(ValidatedProfile {
        name,
        environment: input.environment,
        driver: input.driver,
        host,
        port,
        database_name,
        username,
        ssh_enabled: input.ssh_enabled,
        ssh_host,
        ssh_port,
        ssh_username,
        database_password: input.database_password.filter(|value| !value.is_empty()),
        ssh_password: input.ssh_password.filter(|value| !value.is_empty()),
        clear_database_password: input.clear_database_password,
        clear_ssh_password: input.clear_ssh_password,
    })
}

fn validate_port(value: &str, field: &str, fields: &mut BTreeMap<String, String>) -> u16 {
    match value.parse::<u16>() {
        Ok(port) if port > 0 => port,
        _ => {
            fields.insert(field.into(), "Enter a port from 1 to 65,535.".into());
            0
        }
    }
}

fn valid_host(host: &str) -> bool {
    if host.is_empty() || host.chars().count() > 253 || host.chars().any(char::is_whitespace) {
        return false;
    }
    let unwrapped = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    if unwrapped.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    if unwrapped
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
        && unwrapped.matches('.').count() == 3
    {
        return false;
    }
    let domain = unwrapped.strip_suffix('.').unwrap_or(unwrapped);
    !domain.is_empty()
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
                && label
                    .chars()
                    .next()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
                && label
                    .chars()
                    .last()
                    .is_some_and(|character| character.is_ascii_alphanumeric())
        })
}

fn new_reference(connection: &Connection) -> Result<String, AppError> {
    connection
        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
        .map_err(|_| AppError::storage())
}

fn create_bundle(input: &ValidatedProfile) -> CredentialBundle {
    CredentialBundle {
        database_password: input.database_password.clone(),
        ssh_password: input.ssh_password.clone(),
    }
}

fn apply_secret_changes(
    mut bundle: CredentialBundle,
    input: &ValidatedProfile,
) -> CredentialBundle {
    if input.clear_database_password {
        bundle.database_password = None;
    } else if let Some(password) = &input.database_password {
        bundle.database_password = Some(password.clone());
    }
    if input.clear_ssh_password {
        bundle.ssh_password = None;
    } else if let Some(password) = &input.ssh_password {
        bundle.ssh_password = Some(password.clone());
    }
    bundle
}

fn read_bundle(
    credentials: &dyn CredentialStore,
    reference: &str,
    profile: &ConnectionProfile,
    input: &ValidatedProfile,
) -> Result<CredentialBundle, AppError> {
    let stored = credentials.get(reference)?;
    let Some(stored) = stored else {
        let database_password_can_be_replaced = !profile.has_database_password
            || input.database_password.is_some()
            || input.clear_database_password;
        let ssh_password_can_be_replaced =
            !profile.has_ssh_password || input.ssh_password.is_some() || input.clear_ssh_password;
        if !database_password_can_be_replaced || !ssh_password_can_be_replaced {
            return Err(AppError::credential_missing());
        }
        return Ok(CredentialBundle::default());
    };
    serde_json::from_str(&stored).map_err(|_| AppError::credential_missing())
}

fn insert_profile(
    connection: &Connection,
    id: &str,
    profile: &ValidatedProfile,
    credential_key: Option<&str>,
    bundle: &CredentialBundle,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO connection_profiles (
                id, name, environment, driver, host, port, database_name, username,
                ssh_enabled, ssh_host, ssh_port, ssh_username, credential_key,
                has_database_password, has_ssh_password
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15
             )",
            params![
                id,
                profile.name,
                profile.environment,
                profile.driver,
                profile.host,
                profile.port,
                profile.database_name,
                profile.username,
                profile.ssh_enabled,
                profile.ssh_host,
                profile.ssh_port,
                profile.ssh_username,
                credential_key,
                bundle.database_password.is_some(),
                bundle.ssh_password.is_some(),
            ],
        )
        .map_err(|_| AppError::storage())?;
    Ok(())
}

#[tauri::command]
pub fn list_connection_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionProfile>, AppError> {
    let connection = state.database.lock().map_err(|_| AppError::storage())?;
    list_profiles(&connection)
}

fn list_profiles(connection: &Connection) -> Result<Vec<ConnectionProfile>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {} FROM connection_profiles ORDER BY name COLLATE NOCASE, id",
            profile_columns()
        ))
        .map_err(|_| AppError::storage())?;
    let profiles = statement
        .query_map([], profile_from_row)
        .map_err(|_| AppError::storage())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AppError::storage())?;
    Ok(profiles)
}

#[tauri::command]
pub fn create_connection_profile(
    input: ProfileInput,
    state: State<'_, AppState>,
) -> Result<ProfileWriteResult, AppError> {
    let connection = state.database.lock().map_err(|_| AppError::storage())?;
    create_profile(&connection, &*state.credentials, input)
}

fn create_profile(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    input: ProfileInput,
) -> Result<ProfileWriteResult, AppError> {
    let profile = validate(input)?;
    let bundle = create_bundle(&profile);
    let id = new_reference(connection)?;
    let credential_key = if bundle.is_empty() {
        None
    } else {
        Some(new_reference(connection)?)
    };

    if let Some(reference) = &credential_key {
        let secret = serde_json::to_string(&bundle).map_err(|_| AppError::storage())?;
        credentials.set(reference, &secret)?;
    }
    if let Err(error) = insert_profile(
        connection,
        &id,
        &profile,
        credential_key.as_deref(),
        &bundle,
    ) {
        if credential_key
            .as_deref()
            .is_some_and(|reference| credentials.delete(reference).is_err())
        {
            return Err(AppError::new(
                "credential_cleanup",
                "The profile couldn't be saved. An unused Keychain item may need to be removed in Keychain Access.",
            ));
        }
        return Err(error);
    }

    Ok(ProfileWriteResult {
        profile: get_profile(connection, &id)?,
        warning: None,
    })
}

#[tauri::command]
pub fn update_connection_profile(
    id: String,
    input: ProfileInput,
    state: State<'_, AppState>,
) -> Result<ProfileWriteResult, AppError> {
    let connection = state.database.lock().map_err(|_| AppError::storage())?;
    update_profile(&connection, &*state.credentials, &id, input)
}

fn update_profile(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    id: &str,
    input: ProfileInput,
) -> Result<ProfileWriteResult, AppError> {
    let profile = validate(input)?;
    let existing = get_profile(connection, id)?;
    let old_key = get_credential_key(connection, id)?;
    let secret_changed = profile.database_password.is_some()
        || profile.ssh_password.is_some()
        || profile.clear_database_password
        || profile.clear_ssh_password;

    let (credential_key, bundle, new_key) = if secret_changed {
        let existing_bundle = match old_key.as_deref() {
            Some(reference) => read_bundle(credentials, reference, &existing, &profile)?,
            None => CredentialBundle::default(),
        };
        let bundle = apply_secret_changes(existing_bundle, &profile);
        let next_key = if bundle.is_empty() {
            None
        } else {
            Some(new_reference(connection)?)
        };
        if let Some(reference) = &next_key {
            let secret = serde_json::to_string(&bundle).map_err(|_| AppError::storage())?;
            credentials.set(reference, &secret)?;
        }
        (next_key.clone(), bundle, next_key)
    } else {
        (
            old_key.clone(),
            CredentialBundle {
                database_password: existing.has_database_password.then_some(String::new()),
                ssh_password: existing.has_ssh_password.then_some(String::new()),
            },
            None,
        )
    };

    let update_result = connection.execute(
        "UPDATE connection_profiles SET
            name = ?2, environment = ?3, driver = ?4, host = ?5, port = ?6,
            database_name = ?7, username = ?8, ssh_enabled = ?9, ssh_host = ?10,
            ssh_port = ?11, ssh_username = ?12, credential_key = ?13,
            has_database_password = ?14, has_ssh_password = ?15,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1",
        params![
            id,
            profile.name,
            profile.environment,
            profile.driver,
            profile.host,
            profile.port,
            profile.database_name,
            profile.username,
            profile.ssh_enabled,
            profile.ssh_host,
            profile.ssh_port,
            profile.ssh_username,
            credential_key,
            bundle.database_password.is_some(),
            bundle.ssh_password.is_some(),
        ],
    );
    if update_result.is_err() {
        if let Some(reference) = &new_key
            && credentials.delete(reference).is_err()
        {
            return Err(AppError::new(
                "credential_cleanup",
                "The profile couldn't be saved. An unused Keychain item may need to be removed in Keychain Access.",
            ));
        }
        return Err(AppError::storage());
    }

    let warning = if secret_changed
        && old_key
            .as_deref()
            .is_some_and(|reference| credentials.delete(reference).is_err())
    {
        Some(
            "The profile was saved, but previous credentials couldn't be removed from Keychain."
                .to_owned(),
        )
    } else {
        None
    };
    Ok(ProfileWriteResult {
        profile: get_profile(connection, id)?,
        warning,
    })
}

#[tauri::command]
pub fn duplicate_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<ProfileWriteResult, AppError> {
    let connection = state.database.lock().map_err(|_| AppError::storage())?;
    duplicate_profile(&connection, &*state.credentials, &id)
}

fn duplicate_profile(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    id: &str,
) -> Result<ProfileWriteResult, AppError> {
    let source = get_profile(connection, id)?;
    let old_key = get_credential_key(connection, id)?;
    let bundle = match old_key.as_deref() {
        Some(reference) => match credentials.get(reference)? {
            Some(secret) => {
                serde_json::from_str(&secret).map_err(|_| AppError::credential_missing())?
            }
            None if source.has_database_password || source.has_ssh_password => {
                return Err(AppError::credential_missing());
            }
            None => CredentialBundle::default(),
        },
        None => CredentialBundle::default(),
    };
    let mut input = ProfileInput {
        name: format!("{} copy", source.name),
        environment: source.environment.clone(),
        driver: source.driver.clone(),
        host: source.host.clone(),
        port: source.port.to_string(),
        database_name: source.database_name.clone(),
        username: source.username.clone(),
        ssh_enabled: source.ssh_enabled,
        ssh_host: source.ssh_host.clone(),
        ssh_port: source.ssh_port.to_string(),
        ssh_username: source.ssh_username.clone(),
        database_password: bundle.database_password.clone(),
        ssh_password: bundle.ssh_password.clone(),
        clear_database_password: false,
        clear_ssh_password: false,
    };
    if input.name.chars().count() > 80 {
        input.name = format!("{} copy", source.name.chars().take(75).collect::<String>());
    }
    let profile = validate(input)?;
    let id = new_reference(connection)?;
    let credential_key = if bundle.is_empty() {
        None
    } else {
        Some(new_reference(connection)?)
    };
    if let Some(reference) = &credential_key {
        let secret = serde_json::to_string(&bundle).map_err(|_| AppError::storage())?;
        credentials.set(reference, &secret)?;
    }
    if let Err(error) = insert_profile(
        connection,
        &id,
        &profile,
        credential_key.as_deref(),
        &bundle,
    ) {
        if credential_key
            .as_deref()
            .is_some_and(|reference| credentials.delete(reference).is_err())
        {
            return Err(AppError::new(
                "credential_cleanup",
                "The duplicate couldn't be saved. An unused Keychain item may need to be removed in Keychain Access.",
            ));
        }
        return Err(error);
    }

    Ok(ProfileWriteResult {
        profile: get_profile(connection, &id)?,
        warning: None,
    })
}

#[tauri::command]
pub fn delete_connection_profile(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let mut connection = state.database.lock().map_err(|_| AppError::storage())?;
    delete_profile(&mut connection, &*state.credentials, &id)
}

fn delete_profile(
    connection: &mut Connection,
    credentials: &dyn CredentialStore,
    id: &str,
) -> Result<(), AppError> {
    get_profile(connection, id)?;
    let old_key = get_credential_key(connection, id)?;
    let old_secret = match old_key.as_deref() {
        Some(reference) => credentials.get(reference)?,
        None => None,
    };
    let transaction = connection.transaction().map_err(|_| AppError::storage())?;
    if let Some(reference) = old_key.as_deref()
        && let Err(error) = credentials.delete(reference)
    {
        drop(transaction);
        return Err(error);
    }

    let delete_result = transaction
        .execute("DELETE FROM connection_profiles WHERE id = ?1", [&id])
        .map_err(|_| AppError::storage())
        .and_then(|deleted| {
            if deleted == 0 {
                Err(AppError::new(
                    "not_found",
                    "That saved connection couldn't be found.",
                ))
            } else {
                crate::commands::settings::clear_selected_profile(&transaction, id)?;
                Ok(())
            }
        });

    if delete_result.is_err() {
        drop(transaction);
        restore_secret(credentials, old_key.as_deref(), old_secret.as_deref())?;
        return delete_result;
    }
    if transaction.commit().is_err() {
        restore_secret(credentials, old_key.as_deref(), old_secret.as_deref())?;
        return Err(AppError::storage());
    }
    Ok(())
}

fn restore_secret(
    credentials: &dyn CredentialStore,
    reference: Option<&str>,
    secret: Option<&str>,
) -> Result<(), AppError> {
    if let (Some(reference), Some(secret)) = (reference, secret) {
        credentials.set(reference, secret).map_err(|_| {
            AppError::new(
                "credential_recovery",
                "The profile couldn't be deleted, and its Keychain credential couldn't be restored.",
            )
        })?;
    }
    Ok(())
}

#[tauri::command]
pub fn select_connection_profile(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let connection = state.database.lock().map_err(|_| AppError::storage())?;
    select_profile(&connection, &id)
}

fn select_profile(connection: &Connection, id: &str) -> Result<(), AppError> {
    crate::commands::settings::select_profile(connection, Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrate;
    use std::{collections::HashMap, sync::Mutex};

    #[derive(Default)]
    struct TestCredentials(Mutex<HashMap<String, String>>);

    impl CredentialStore for TestCredentials {
        fn set(&self, reference: &str, value: &str) -> Result<(), AppError> {
            self.0
                .lock()
                .map_err(|_| AppError::credential_access())?
                .insert(reference.into(), value.into());
            Ok(())
        }

        fn get(&self, reference: &str) -> Result<Option<String>, AppError> {
            Ok(self
                .0
                .lock()
                .map_err(|_| AppError::credential_access())?
                .get(reference)
                .cloned())
        }

        fn delete(&self, reference: &str) -> Result<(), AppError> {
            self.0
                .lock()
                .map_err(|_| AppError::credential_access())?
                .remove(reference);
            Ok(())
        }
    }

    fn input() -> ProfileInput {
        ProfileInput {
            name: "Local database".into(),
            environment: "local".into(),
            driver: "postgres".into(),
            host: "localhost".into(),
            port: "5432".into(),
            database_name: "demo".into(),
            username: "user".into(),
            ssh_enabled: false,
            ssh_host: String::new(),
            ssh_port: "22".into(),
            ssh_username: String::new(),
            database_password: None,
            ssh_password: None,
            clear_database_password: false,
            clear_ssh_password: false,
        }
    }

    #[test]
    fn validates_hostnames_and_ip_addresses() {
        assert!(valid_host("db.example.com"));
        assert!(valid_host("127.0.0.1"));
        assert!(valid_host("::1"));
        assert!(!valid_host("-invalid.example"));
        assert!(!valid_host("host name"));
    }

    #[test]
    fn reports_invalid_ports_and_required_fields_by_field() {
        let mut invalid = input();
        invalid.name = " ".into();
        invalid.port = "65536".into();
        invalid.host = "bad host".into();

        let error = validate(invalid)
            .err()
            .expect("invalid profile should fail");

        assert_eq!(error.kind, "validation");
        let fields = error.fields.unwrap();
        assert!(fields.contains_key("name"));
        assert!(fields.contains_key("port"));
        assert!(fields.contains_key("host"));
    }

    #[test]
    fn accepts_valid_profile_without_secrets_or_ssh() {
        let validated = validate(input()).unwrap();
        assert_eq!(validated.driver, "postgres");
        assert_eq!(validated.port, 5432);
        assert!(validated.database_password.is_none());
        assert!(!validated.ssh_enabled);
    }

    #[test]
    fn profile_crud_and_selection_keep_secrets_out_of_sqlite_and_responses() {
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();
        let credentials = TestCredentials::default();

        let mut create = input();
        create.database_password = Some("database-secret".into());
        create.ssh_enabled = true;
        create.ssh_host = "bastion.example.com".into();
        create.ssh_username = "tunnel-user".into();
        create.ssh_password = Some("ssh-secret".into());
        let created = create_profile(&connection, &credentials, create).unwrap();
        let original_id = created.profile.id.clone();
        let original_key = get_credential_key(&connection, &original_id)
            .unwrap()
            .unwrap();

        assert!(created.profile.has_database_password);
        assert!(created.profile.has_ssh_password);
        let serialized = serde_json::to_string(&created.profile).unwrap();
        assert!(!serialized.contains("database-secret"));
        assert!(!serialized.contains("ssh-secret"));
        let stored_key: String = connection
            .query_row(
                "SELECT credential_key FROM connection_profiles WHERE id = ?1",
                [&original_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_ne!(stored_key, "database-secret");
        let stored_profile: String = connection
            .query_row(
                "SELECT id || name || environment || driver || host || port ||
                        database_name || username || auth_mode || tls_mode ||
                        ssh_enabled || ssh_host || ssh_port || ssh_username ||
                        ssh_auth_mode || credential_key || has_database_password ||
                        has_ssh_password
                 FROM connection_profiles WHERE id = ?1",
                [&original_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!stored_profile.contains("database-secret"));
        assert!(!stored_profile.contains("ssh-secret"));
        assert!(
            credentials
                .get(&original_key)
                .unwrap()
                .unwrap()
                .contains("database-secret")
        );

        let mut metadata_update = input();
        metadata_update.name = "Renamed profile".into();
        metadata_update.ssh_enabled = true;
        metadata_update.ssh_host = "bastion.example.com".into();
        metadata_update.ssh_username = "tunnel-user".into();
        let updated =
            update_profile(&connection, &credentials, &original_id, metadata_update).unwrap();
        assert_eq!(updated.profile.name, "Renamed profile");
        assert!(credentials.get(&original_key).unwrap().is_some());

        let mut secret_update = input();
        secret_update.name = "Renamed profile".into();
        secret_update.database_password = Some("replacement-secret".into());
        secret_update.ssh_enabled = true;
        secret_update.ssh_host = "bastion.example.com".into();
        secret_update.ssh_username = "tunnel-user".into();
        let updated =
            update_profile(&connection, &credentials, &original_id, secret_update).unwrap();
        let replacement_key = get_credential_key(&connection, &original_id)
            .unwrap()
            .unwrap();
        assert_ne!(replacement_key, original_key);
        assert!(credentials.get(&original_key).unwrap().is_none());
        assert!(
            credentials
                .get(&replacement_key)
                .unwrap()
                .unwrap()
                .contains("replacement-secret")
        );
        assert!(updated.profile.has_ssh_password);

        let duplicate = duplicate_profile(&connection, &credentials, &original_id).unwrap();
        let duplicate_key = get_credential_key(&connection, &duplicate.profile.id)
            .unwrap()
            .unwrap();
        assert_ne!(duplicate_key, replacement_key);
        assert_eq!(duplicate.profile.name, "Renamed profile copy");
        assert!(credentials.get(&duplicate_key).unwrap().is_some());
        select_profile(&connection, &original_id).unwrap();
        assert_eq!(
            crate::commands::settings::read_settings(&connection)
                .unwrap()
                .selected_profile_id,
            Some(original_id.clone())
        );

        let mut connection = connection;
        delete_profile(&mut connection, &credentials, &original_id).unwrap();
        assert!(credentials.get(&replacement_key).unwrap().is_none());
        assert_eq!(
            crate::commands::settings::read_settings(&connection)
                .unwrap()
                .selected_profile_id,
            None
        );
        assert_eq!(list_profiles(&connection).unwrap().len(), 1);
    }

    #[test]
    fn duplicating_a_profile_with_a_missing_saved_secret_returns_a_clear_error() {
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();
        let credentials = TestCredentials::default();
        let mut create = input();
        create.database_password = Some("not-on-disk".into());
        let created = create_profile(&connection, &credentials, create).unwrap();
        let key = get_credential_key(&connection, &created.profile.id)
            .unwrap()
            .unwrap();
        credentials.delete(&key).unwrap();

        let error = duplicate_profile(&connection, &credentials, &created.profile.id)
            .expect_err("missing credentials should be reported");
        assert_eq!(error.kind, "credential_missing");
        assert!(!error.message.contains("not-on-disk"));
    }

    #[test]
    fn profiles_and_selection_survive_reopening_the_sqlite_database() {
        let path = std::env::temp_dir().join(format!(
            "thabi-studio-persistence-test-{}.sqlite3",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let credentials = TestCredentials::default();
        let id = {
            let connection = Connection::open(&path).unwrap();
            migrate(&connection).unwrap();
            let profile = create_profile(&connection, &credentials, input()).unwrap();
            select_profile(&connection, &profile.profile.id).unwrap();
            profile.profile.id
        };

        let connection = Connection::open(&path).unwrap();
        migrate(&connection).unwrap();
        let profiles = list_profiles(&connection).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, id);
        assert_eq!(
            crate::commands::settings::read_settings(&connection)
                .unwrap()
                .selected_profile_id,
            Some(id)
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }
}
