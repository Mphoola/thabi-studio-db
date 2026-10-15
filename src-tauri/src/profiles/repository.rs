//! Profile storage. Every function takes the SQLite connection and the
//! credential store explicitly so tests can use in-memory versions.

use super::{ConnectionProfile, ProfileInput, ProfileWriteResult, ValidatedProfile, validate};
use crate::{
    credentials::CredentialStore,
    database::{ConnectParams, Engine, TlsMode},
    errors::AppError,
    ssh::{SshAuth, SshParams},
};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CredentialBundle {
    database_password: Option<String>,
    ssh_password: Option<String>,
    #[serde(default)]
    ssh_key_passphrase: Option<String>,
}

impl CredentialBundle {
    fn is_empty(&self) -> bool {
        self.database_password.is_none()
            && self.ssh_password.is_none()
            && self.ssh_key_passphrase.is_none()
    }
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
        tls_ca_path: row.get("tls_ca_path")?,
        ssh_enabled: row.get::<_, i64>("ssh_enabled")? != 0,
        ssh_host: row.get("ssh_host")?,
        ssh_port: row.get("ssh_port")?,
        ssh_username: row.get("ssh_username")?,
        ssh_auth_mode: row.get("ssh_auth_mode")?,
        ssh_key_path: row.get("ssh_key_path")?,
        has_ssh_key_passphrase: row.get::<_, i64>("has_ssh_key_passphrase")? != 0,
        has_database_password: row.get::<_, i64>("has_database_password")? != 0,
        has_ssh_password: row.get::<_, i64>("has_ssh_password")? != 0,
        safe_mode: row.get("safe_mode")?,
        group_name: row.get("group_name")?,
        colour: row.get("colour")?,
        sort_order: row.get("sort_order")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        last_connected_at: row.get("last_connected_at")?,
    })
}

fn profile_columns() -> &'static str {
    "id, name, environment, driver, host, port, database_name, username,
     auth_mode, tls_mode, tls_ca_path, ssh_enabled, ssh_host, ssh_port, ssh_username,
     ssh_auth_mode, ssh_key_path, has_database_password, has_ssh_password,
     has_ssh_key_passphrase, safe_mode, group_name, colour, sort_order,
     created_at, updated_at, last_connected_at"
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
        .ok_or_else(AppError::not_found)
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
        .ok_or_else(AppError::not_found)
}

fn saved_bundle(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    profile: &ConnectionProfile,
) -> Result<CredentialBundle, AppError> {
    let bundle = match get_credential_key(connection, &profile.id)?.as_deref() {
        Some(reference) => match credentials.get(reference)? {
            Some(secret) => serde_json::from_str::<CredentialBundle>(&secret)
                .map_err(|_| AppError::credential_missing())?,
            None if profile.has_database_password
                || profile.has_ssh_password
                || profile.has_ssh_key_passphrase =>
            {
                return Err(AppError::credential_missing());
            }
            None => CredentialBundle::default(),
        },
        None => CredentialBundle::default(),
    };
    if profile.has_database_password && bundle.database_password.is_none() {
        return Err(AppError::credential_missing());
    }
    Ok(bundle)
}

/// The connection fields shared by saved profiles and unsaved form input.
struct Target<'a> {
    driver: &'a str,
    host: &'a str,
    port: u16,
    database_name: &'a str,
    username: &'a str,
    tls_mode: &'a str,
    tls_ca_path: &'a str,
}

fn params_from(target: Target<'_>, password: Option<String>) -> Result<ConnectParams, AppError> {
    Ok(ConnectParams {
        engine: Engine::from_driver(target.driver)?,
        host: target.host.to_owned(),
        port: target.port,
        database: Some(target.database_name.to_owned()).filter(|name| !name.is_empty()),
        username: target.username.to_owned(),
        password,
        tls: TlsMode::from_setting(target.tls_mode)?,
        ca_path: Some(PathBuf::from(target.tls_ca_path)).filter(|_| !target.tls_ca_path.is_empty()),
        ssh: None,
        connect_to: None,
        read_only: false,
    })
}

/// SSH settings shared by saved profiles and unsaved form input.
struct SshTarget<'a> {
    host: &'a str,
    port: u16,
    username: &'a str,
    auth_mode: &'a str,
    key_path: &'a str,
    password: Option<String>,
    passphrase: Option<String>,
}

/// Builds the params for a jump host named in `~/.ssh/config`. It signs in the
/// same way the target does, which is what almost every ProxyJump setup uses.
fn jump_params(
    connection: &Connection,
    alias: &str,
    auth: &SshAuth,
) -> Result<SshParams, AppError> {
    let resolved = crate::ssh::config::resolve_user_alias(alias);
    let host = resolved
        .as_ref()
        .and_then(|config| config.hostname.clone())
        .unwrap_or_else(|| alias.to_owned());
    let username = resolved
        .as_ref()
        .and_then(|config| config.user.clone())
        .unwrap_or_else(|| match auth {
            SshAuth::Password(_) | SshAuth::KeyFile { .. } => String::new(),
            SshAuth::Agent => std::env::var("USER").unwrap_or_default(),
        });
    let port = resolved
        .as_ref()
        .and_then(|config| config.port)
        .unwrap_or(22);
    let auth = match auth {
        SshAuth::Password(password) => SshAuth::Password(password.clone()),
        SshAuth::KeyFile { path, passphrase } => {
            // The jump host's own identity file wins when the config names one.
            let path = resolved
                .as_ref()
                .and_then(|config| config.identity_file.clone())
                .map(PathBuf::from)
                .unwrap_or_else(|| path.clone());
            SshAuth::KeyFile {
                path,
                passphrase: passphrase.clone(),
            }
        }
        SshAuth::Agent => SshAuth::Agent,
    };
    Ok(SshParams {
        host: host.clone(),
        port,
        username,
        auth,
        known_keys: crate::ssh::known_host_keys(connection, &host, port)?,
        proxy_jump: None,
    })
}

fn ssh_params(connection: &Connection, target: SshTarget<'_>) -> Result<SshParams, AppError> {
    // A `Host` alias in the user's own SSH config supplies the same settings
    // ssh would use, including one ProxyJump hop.
    let alias = crate::ssh::config::resolve_user_alias(target.host);
    let host = alias
        .as_ref()
        .and_then(|config| config.hostname.clone())
        .unwrap_or_else(|| target.host.to_owned());
    let username = alias
        .as_ref()
        .and_then(|config| config.user.clone())
        .unwrap_or_else(|| target.username.to_owned());
    let port = alias
        .as_ref()
        .and_then(|config| config.port)
        .unwrap_or(target.port);
    let key_path = if target.auth_mode == "key" {
        alias
            .as_ref()
            .and_then(|config| config.identity_file.clone())
            .unwrap_or_else(|| target.key_path.to_owned())
    } else {
        target.key_path.to_owned()
    };

    let auth = match target.auth_mode {
        "password" => SshAuth::Password(target.password.ok_or_else(|| {
            AppError::new(
                "ssh_credentials",
                "Enter the SSH password in the connection settings.",
            )
        })?),
        "key" => SshAuth::KeyFile {
            path: PathBuf::from(&key_path),
            passphrase: target.passphrase,
        },
        "agent" => SshAuth::Agent,
        _ => return Err(AppError::new("validation", "Choose how to sign in to SSH.")),
    };
    let proxy_jump = match alias.as_ref().map(|config| config.proxy_jump.clone()) {
        Some(jump) if !jump.is_empty() => Some(Box::new(jump_params(connection, &jump, &auth)?)),
        _ => None,
    };
    Ok(SshParams {
        host: host.clone(),
        port,
        username,
        auth,
        known_keys: crate::ssh::known_host_keys(connection, &host, port)?,
        proxy_jump,
    })
}

/// Everything needed to connect to a saved profile, passwords included.
pub fn connect_params(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    id: &str,
) -> Result<(ConnectionProfile, ConnectParams), AppError> {
    let profile = get_profile(connection, id)?;
    let bundle = saved_bundle(connection, credentials, &profile)?;
    let ssh = if profile.ssh_enabled {
        Some(ssh_params(
            connection,
            SshTarget {
                host: &profile.ssh_host,
                port: profile.ssh_port,
                username: &profile.ssh_username,
                auth_mode: &profile.ssh_auth_mode,
                key_path: &profile.ssh_key_path,
                password: bundle.ssh_password.clone(),
                passphrase: bundle.ssh_key_passphrase.clone(),
            },
        )?)
    } else {
        None
    };
    let mut params = params_from(
        Target {
            driver: &profile.driver,
            host: &profile.host,
            port: profile.port,
            database_name: &profile.database_name,
            username: &profile.username,
            tls_mode: &profile.tls_mode,
            tls_ca_path: &profile.tls_ca_path,
        },
        bundle.database_password,
    )?;
    params.ssh = ssh;
    params.read_only = profile.safe_mode == "read-only";
    Ok((profile, params))
}

/// Connection settings for unsaved form values, used by Test Connection.
/// A typed secret wins; otherwise an edited profile's saved secret is used
/// unless the form asks to remove it.
pub fn connect_params_for_input(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    profile_id: Option<&str>,
    input: &ValidatedProfile,
) -> Result<ConnectParams, AppError> {
    let saved = match profile_id {
        Some(id) => {
            let profile = get_profile(connection, id)?;
            saved_bundle(connection, credentials, &profile)?
        }
        None => CredentialBundle::default(),
    };
    let pick = |typed: &Option<String>, clear: bool, saved: Option<String>| match typed {
        Some(value) => Some(value.clone()),
        None if !clear => saved,
        None => None,
    };
    let password = pick(
        &input.database_password,
        input.clear_database_password,
        saved.database_password,
    );
    let ssh = if input.ssh_enabled {
        Some(ssh_params(
            connection,
            SshTarget {
                host: &input.ssh_host,
                port: input.ssh_port,
                username: &input.ssh_username,
                auth_mode: &input.ssh_auth_mode,
                key_path: &input.ssh_key_path,
                password: pick(
                    &input.ssh_password,
                    input.clear_ssh_password,
                    saved.ssh_password,
                ),
                passphrase: pick(
                    &input.ssh_key_passphrase,
                    input.clear_ssh_key_passphrase,
                    saved.ssh_key_passphrase,
                ),
            },
        )?)
    } else {
        None
    };
    let mut params = params_from(
        Target {
            driver: &input.driver,
            host: &input.host,
            port: input.port,
            database_name: &input.database_name,
            username: &input.username,
            tls_mode: &input.tls_mode,
            tls_ca_path: &input.tls_ca_path,
        },
        password,
    )?;
    params.ssh = ssh;
    Ok(params)
}

pub fn mark_connected(connection: &Connection, id: &str) -> Result<(), AppError> {
    connection
        .execute(
            "UPDATE connection_profiles SET last_connected_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [id],
        )
        .map_err(|_| AppError::storage())?;
    Ok(())
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
        ssh_key_passphrase: input.ssh_key_passphrase.clone(),
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
    if input.clear_ssh_key_passphrase {
        bundle.ssh_key_passphrase = None;
    } else if let Some(passphrase) = &input.ssh_key_passphrase {
        bundle.ssh_key_passphrase = Some(passphrase.clone());
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
        let passphrase_can_be_replaced = !profile.has_ssh_key_passphrase
            || input.ssh_key_passphrase.is_some()
            || input.clear_ssh_key_passphrase;
        if !database_password_can_be_replaced
            || !ssh_password_can_be_replaced
            || !passphrase_can_be_replaced
        {
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
    sort_order: i64,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT INTO connection_profiles (
                id, name, environment, driver, host, port, database_name, username,
                ssh_enabled, ssh_host, ssh_port, ssh_username, credential_key,
                has_database_password, has_ssh_password, tls_mode, tls_ca_path,
                ssh_auth_mode, ssh_key_path, has_ssh_key_passphrase, safe_mode,
                group_name, colour, sort_order
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21, ?22, ?23, ?24
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
                profile.tls_mode,
                profile.tls_ca_path,
                profile.ssh_auth_mode,
                profile.ssh_key_path,
                bundle.ssh_key_passphrase.is_some(),
                profile.safe_mode,
                profile.group_name,
                profile.colour,
                sort_order,
            ],
        )
        .map_err(|_| AppError::storage())?;
    Ok(())
}

pub fn list_profiles(connection: &Connection) -> Result<Vec<ConnectionProfile>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {} FROM connection_profiles
             ORDER BY (group_name = ''), group_name COLLATE NOCASE, sort_order,
                      name COLLATE NOCASE, id",
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

/// Sets the manual order from the ids in the order given. Unknown ids are
/// ignored; profiles left out of the list keep their current number, so the
/// ordering stays stable when the user reorders one group at a time.
pub fn reorder_profiles(connection: &mut Connection, ids: &[String]) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(|_| AppError::storage())?;
    for (index, id) in ids.iter().enumerate() {
        transaction
            .execute(
                "UPDATE connection_profiles SET sort_order = ?2, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![id, index as i64],
            )
            .map_err(|_| AppError::storage())?;
    }
    transaction.commit().map_err(|_| AppError::storage())?;
    Ok(())
}

/// A new connection goes to the end of its group so it appears last there.
fn next_sort_order(connection: &Connection, group_name: &str) -> Result<i64, AppError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM connection_profiles
             WHERE group_name = ?1 COLLATE NOCASE",
            [group_name],
            |row| row.get(0),
        )
        .map_err(|_| AppError::storage())
}

pub fn create_profile(
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
        next_sort_order(connection, &profile.group_name)?,
    ) {
        if credential_key
            .as_deref()
            .is_some_and(|reference| credentials.delete(reference).is_err())
        {
            return Err(AppError::new(
                "credential_cleanup",
                "The connection couldn't be saved. An unused saved password may need to be removed from the system credential store.",
            ));
        }
        return Err(error);
    }

    Ok(ProfileWriteResult {
        profile: get_profile(connection, &id)?,
        warning: None,
    })
}

#[cfg(test)]
pub fn update_profile(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    id: &str,
    input: ProfileInput,
) -> Result<ProfileWriteResult, AppError> {
    let profile = validate(input)?;
    update_validated_profile(connection, credentials, id, profile)
}

pub fn update_validated_profile(
    connection: &Connection,
    credentials: &dyn CredentialStore,
    id: &str,
    profile: ValidatedProfile,
) -> Result<ProfileWriteResult, AppError> {
    let existing = get_profile(connection, id)?;
    let old_key = get_credential_key(connection, id)?;
    let secret_changed = profile.database_password.is_some()
        || profile.ssh_password.is_some()
        || profile.ssh_key_passphrase.is_some()
        || profile.clear_ssh_key_passphrase
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
                ssh_key_passphrase: existing.has_ssh_key_passphrase.then_some(String::new()),
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
            tls_mode = ?16, tls_ca_path = ?17, ssh_auth_mode = ?18, ssh_key_path = ?19,
            has_ssh_key_passphrase = ?20, safe_mode = ?21,
            group_name = ?22, colour = ?23,
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
            profile.tls_mode,
            profile.tls_ca_path,
            profile.ssh_auth_mode,
            profile.ssh_key_path,
            bundle.ssh_key_passphrase.is_some(),
            profile.safe_mode,
            profile.group_name,
            profile.colour,
        ],
    );
    if update_result.is_err() {
        if let Some(reference) = &new_key
            && credentials.delete(reference).is_err()
        {
            return Err(AppError::new(
                "credential_cleanup",
                "The connection couldn't be saved. An unused saved password may need to be removed from the system credential store.",
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
            "The connection was saved, but its previous password couldn't be removed from the system credential store."
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

pub fn duplicate_profile(
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
            None if source.has_database_password
                || source.has_ssh_password
                || source.has_ssh_key_passphrase =>
            {
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
        tls_mode: source.tls_mode.clone(),
        tls_ca_path: source.tls_ca_path.clone(),
        ssh_enabled: source.ssh_enabled,
        ssh_host: source.ssh_host.clone(),
        ssh_port: source.ssh_port.to_string(),
        ssh_username: source.ssh_username.clone(),
        ssh_auth_mode: source.ssh_auth_mode.clone(),
        ssh_key_path: source.ssh_key_path.clone(),
        database_password: bundle.database_password.clone(),
        ssh_password: bundle.ssh_password.clone(),
        clear_database_password: false,
        clear_ssh_password: false,
        ssh_key_passphrase: bundle.ssh_key_passphrase.clone(),
        clear_ssh_key_passphrase: false,
        safe_mode: source.safe_mode.clone(),
        group_name: source.group_name.clone(),
        colour: source.colour.clone(),
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
        next_sort_order(connection, &profile.group_name)?,
    ) {
        if credential_key
            .as_deref()
            .is_some_and(|reference| credentials.delete(reference).is_err())
        {
            return Err(AppError::new(
                "credential_cleanup",
                "The copy couldn't be saved. An unused saved password may need to be removed from the system credential store.",
            ));
        }
        return Err(error);
    }

    Ok(ProfileWriteResult {
        profile: get_profile(connection, &id)?,
        warning: None,
    })
}

pub fn delete_profile(
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
                crate::settings::clear_selected_profile(&transaction, id)?;
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
                "The connection couldn't be deleted, and its saved password couldn't be restored.",
            )
        })?;
    }
    Ok(())
}

pub fn select_profile(connection: &Connection, id: &str) -> Result<(), AppError> {
    crate::settings::select_profile(connection, Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profiles::validation::tests::input;
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

    #[test]
    fn connection_parameters_load_password_only_from_secure_storage() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();
        let mut profile_input = input();
        profile_input.database_password = Some("database-secret".into());
        let saved = create_profile(&connection, &credentials, profile_input).unwrap();

        let (profile, params) =
            connect_params(&connection, &credentials, &saved.profile.id).unwrap();

        assert_eq!(params.password.as_deref(), Some("database-secret"));
        assert!(!format!("{params:?}").contains("database-secret"));
        assert!(
            !serde_json::to_string(&profile)
                .unwrap()
                .contains("database-secret")
        );
    }

    #[test]
    fn connection_parameters_report_missing_saved_password() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();
        let mut profile_input = input();
        profile_input.database_password = Some("database-secret".into());
        let saved = create_profile(&connection, &credentials, profile_input).unwrap();
        let key = get_credential_key(&connection, &saved.profile.id)
            .unwrap()
            .unwrap();
        credentials.delete(&key).unwrap();

        let error = match connect_params(&connection, &credentials, &saved.profile.id) {
            Ok(_) => panic!("missing saved password should fail"),
            Err(error) => error,
        };

        assert_eq!(error.kind, "credential_missing");
        assert!(!error.message.contains("database-secret"));
    }

    #[test]
    fn profile_crud_and_selection_keep_secrets_out_of_sqlite_and_responses() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
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
            crate::settings::read_settings(&connection)
                .unwrap()
                .selected_profile_id,
            Some(original_id.clone())
        );

        let mut connection = connection;
        delete_profile(&mut connection, &credentials, &original_id).unwrap();
        assert!(credentials.get(&replacement_key).unwrap().is_none());
        assert_eq!(
            crate::settings::read_settings(&connection)
                .unwrap()
                .selected_profile_id,
            None
        );
        assert_eq!(list_profiles(&connection).unwrap().len(), 1);
    }

    #[test]
    fn duplicating_a_profile_with_a_missing_saved_secret_returns_a_clear_error() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
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
            let mut connection = Connection::open(&path).unwrap();
            migrate(&mut connection).unwrap();
            let profile = create_profile(&connection, &credentials, input()).unwrap();
            select_profile(&connection, &profile.profile.id).unwrap();
            profile.profile.id
        };

        let mut connection = Connection::open(&path).unwrap();
        migrate(&mut connection).unwrap();
        let profiles = list_profiles(&connection).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, id);
        assert_eq!(
            crate::settings::read_settings(&connection)
                .unwrap()
                .selected_profile_id,
            Some(id)
        );
        drop(connection);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn testing_form_values_prefers_a_typed_password_then_the_saved_one() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();
        let mut create = input();
        create.database_password = Some("saved-secret".into());
        let saved = create_profile(&connection, &credentials, create).unwrap();
        let id = saved.profile.id.as_str();

        let unchanged = validate(input()).unwrap();
        let params =
            connect_params_for_input(&connection, &credentials, Some(id), &unchanged).unwrap();
        assert_eq!(params.password.as_deref(), Some("saved-secret"));

        let mut typed = input();
        typed.database_password = Some("typed-secret".into());
        let typed = validate(typed).unwrap();
        let params = connect_params_for_input(&connection, &credentials, Some(id), &typed).unwrap();
        assert_eq!(params.password.as_deref(), Some("typed-secret"));

        let mut cleared = input();
        cleared.clear_database_password = true;
        let cleared = validate(cleared).unwrap();
        let params =
            connect_params_for_input(&connection, &credentials, Some(id), &cleared).unwrap();
        assert!(params.password.is_none());

        let new_profile =
            connect_params_for_input(&connection, &credentials, None, &unchanged).unwrap();
        assert!(new_profile.password.is_none());
    }

    #[test]
    fn ssh_profiles_connect_through_a_tunnel_with_saved_secrets() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();
        let mut create = input();
        create.ssh_enabled = true;
        create.ssh_host = "bastion.example.com".into();
        create.ssh_username = "tunnel".into();
        create.ssh_password = Some("ssh-secret".into());
        let saved = create_profile(&connection, &credentials, create.clone()).unwrap();

        let (_, params) = connect_params(&connection, &credentials, &saved.profile.id).unwrap();
        let ssh = params.ssh.expect("ssh params");
        assert_eq!(ssh.host, "bastion.example.com");
        assert!(matches!(ssh.auth, SshAuth::Password(ref p) if p == "ssh-secret"));
        assert!(!format!("{ssh:?}").contains("ssh-secret"));

        // Unsaved form values without an SSH password can't connect.
        create.ssh_password = None;
        let error =
            connect_params_for_input(&connection, &credentials, None, &validate(create).unwrap())
                .unwrap_err();
        assert_eq!(error.kind, "ssh_credentials");
    }

    #[test]
    fn connecting_records_the_time_and_tls_settings_round_trip() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();
        let mut create = input();
        create.tls_mode = "require".into();
        let saved = create_profile(&connection, &credentials, create).unwrap();
        assert_eq!(saved.profile.tls_mode, "require");
        assert!(saved.profile.last_connected_at.is_none());

        mark_connected(&connection, &saved.profile.id).unwrap();
        let (profile, params) =
            connect_params(&connection, &credentials, &saved.profile.id).unwrap();
        assert!(profile.last_connected_at.is_some());
        assert_eq!(params.tls, TlsMode::EncryptOnly);
    }

    #[test]
    fn safe_mode_is_stored_on_the_profile_and_drives_read_only_sessions() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();

        let mut create = input();
        create.safe_mode = "read-only".into();
        let saved = create_profile(&connection, &credentials, create.clone()).unwrap();
        assert_eq!(saved.profile.safe_mode, "read-only");
        let (_, params) = connect_params(&connection, &credentials, &saved.profile.id).unwrap();
        assert!(params.read_only);

        // A copy keeps the mode, and so does an edit that changes it.
        let copy = duplicate_profile(&connection, &credentials, &saved.profile.id).unwrap();
        assert_eq!(copy.profile.safe_mode, "read-only");

        create.safe_mode = "confirm-writes".into();
        let updated = update_profile(&connection, &credentials, &saved.profile.id, create).unwrap();
        assert_eq!(updated.profile.safe_mode, "confirm-writes");
        let (_, params) = connect_params(&connection, &credentials, &saved.profile.id).unwrap();
        assert!(!params.read_only);
    }

    #[test]
    fn production_profiles_start_on_confirm_writes_and_others_on_off() {
        let mut connection = Connection::open_in_memory().unwrap();
        // A database from before safe modes existed, with a Production profile
        // already in it: upgrading it must keep today's behaviour.
        let migrations = crate::storage::all_migrations();
        let before_safe_mode = migrations
            .iter()
            .position(|sql| sql.contains("ADD COLUMN safe_mode"))
            .expect("the safe mode migration is registered");
        connection
            .execute_batch(&migrations[..before_safe_mode].join("\n"))
            .unwrap();
        connection
            .pragma_update(None, "user_version", before_safe_mode as i64)
            .unwrap();
        connection
            .execute(
                "INSERT INTO connection_profiles (id, name, environment, driver, host, port, username)
                 VALUES ('prod', 'Production', 'production', 'postgres', 'localhost', 5432, 'user')",
                [],
            )
            .unwrap();
        migrate(&mut connection).unwrap();

        let migrated = get_profile(&connection, "prod").unwrap();
        assert_eq!(migrated.safe_mode, "confirm-writes");

        let credentials = TestCredentials::default();
        let created = create_profile(&connection, &credentials, input()).unwrap();
        assert_eq!(created.profile.safe_mode, "off");
    }

    #[test]
    fn groups_and_colours_round_trip_and_dragging_reorders() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();

        let mut work = input();
        work.name = "Work laptop".into();
        work.group_name = "Work".into();
        work.colour = "purple".into();
        let first = create_profile(&connection, &credentials, work).unwrap();
        assert_eq!(first.profile.group_name, "Work");
        assert_eq!(first.profile.colour, "purple");

        let mut home = input();
        home.name = "Home box".into();
        home.group_name = "work".into(); // Same group, different case.
        home.colour = "green".into();
        let second = create_profile(&connection, &credentials, home).unwrap();

        let mut loose = input();
        loose.name = "Scratch".into();
        let third = create_profile(&connection, &credentials, loose).unwrap();
        assert_eq!(third.profile.group_name, "");

        // Both land in the same group, ordered by creation, ungrouped last.
        let listed = list_profiles(&connection).unwrap();
        assert_eq!(
            listed.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Work laptop", "Home box", "Scratch"]
        );

        // Dragging Home box above Work laptop is remembered.
        let ids = vec![second.profile.id.clone(), first.profile.id.clone()];
        reorder_profiles(&mut connection, &ids).unwrap();
        let listed = list_profiles(&connection).unwrap();
        assert_eq!(
            listed.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
            ["Home box", "Work laptop", "Scratch"]
        );

        // An unknown id in the list is ignored rather than failing the reorder.
        let ids = vec!["nope".to_owned(), third.profile.id.clone()];
        reorder_profiles(&mut connection, &ids).unwrap();
        assert_eq!(list_profiles(&connection).unwrap().len(), 3);

        // The copy keeps the group and colour as they were written.
        let copy = duplicate_profile(&connection, &credentials, &second.profile.id).unwrap();
        assert_eq!(copy.profile.group_name, "work");
        assert_eq!(copy.profile.colour, "green");
    }

    #[test]
    fn an_unknown_colour_or_long_group_name_is_rejected_by_field() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();

        let mut bad_colour = input();
        bad_colour.colour = "chartreuse".into();
        let error = create_profile(&connection, &credentials, bad_colour)
            .expect_err("unknown colour should be rejected");
        assert!(error.fields.unwrap().contains_key("colour"));

        let mut long_group = input();
        long_group.group_name = "x".repeat(61);
        let error = create_profile(&connection, &credentials, long_group)
            .expect_err("a long group name should be rejected");
        assert!(error.fields.unwrap().contains_key("groupName"));
    }

    #[test]
    fn an_unknown_safe_mode_is_rejected_by_field() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let credentials = TestCredentials::default();
        let mut create = input();
        create.safe_mode = "yolo".into();
        let error = create_profile(&connection, &credentials, create)
            .expect_err("unknown safe mode should be rejected");
        assert_eq!(error.kind, "validation");
        assert!(error.fields.unwrap().contains_key("safeMode"));
    }
}
