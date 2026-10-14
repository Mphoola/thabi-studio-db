use crate::errors::AppError;
use rusqlite::Connection;

const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_local_profiles.sql");
const PROFILE_SECURITY_MIGRATION: &str =
    include_str!("../migrations/0002_profile_security_settings.sql");
const CURRENT_VERSION: i64 = 2;

pub fn migrate(connection: &Connection) -> Result<(), AppError> {
    let mut version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|_| AppError::storage())?;
    if version > CURRENT_VERSION {
        return Err(AppError::new(
            "storage",
            "This database was created by a newer version of Thabi Studio.",
        ));
    }
    while version < CURRENT_VERSION {
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| AppError::storage())?;
        let migration = match version {
            0 => INITIAL_MIGRATION,
            1 => PROFILE_SECURITY_MIGRATION,
            _ => return Err(AppError::storage()),
        };
        execute_migration(&transaction, migration)?;
        transaction
            .pragma_update(None, "user_version", version + 1)
            .map_err(|_| AppError::storage())?;
        transaction.commit().map_err(|_| AppError::storage())?;
        version += 1;
    }
    Ok(())
}

fn execute_migration(connection: &Connection, migration: &str) -> Result<(), AppError> {
    for statement in migration
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
    {
        if let Some(column) = statement
            .strip_prefix("ALTER TABLE connection_profiles")
            .and_then(|rest| rest.trim().strip_prefix("ADD COLUMN"))
            .and_then(|rest| rest.split_whitespace().next())
        {
            let exists: bool = connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM pragma_table_info('connection_profiles')
                        WHERE name = ?1
                     )",
                    [column],
                    |row| row.get(0),
                )
                .map_err(|_| AppError::storage())?;
            if exists {
                continue;
            }
        }
        connection
            .execute_batch(statement)
            .map_err(|_| AppError::storage())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_migration_creates_profile_and_settings_tables() {
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();

        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let profile_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'connection_profiles'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let settings_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0))
            .unwrap();

        assert_eq!(version, 2);
        assert_eq!(profile_count, 1);
        assert_eq!(settings_count, 4);
    }

    #[test]
    fn migration_is_idempotent() {
        let connection = Connection::open_in_memory().unwrap();
        migrate(&connection).unwrap();
        migrate(&connection).unwrap();
    }

    #[test]
    fn upgrades_an_existing_version_one_database_without_losing_profiles() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute(
                "INSERT INTO connection_profiles (id, name, environment, driver, host, port, username)
                 VALUES ('existing', 'Existing profile', 'local', 'postgres', 'localhost', 5432, 'user')",
                [],
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();

        migrate(&connection).unwrap();

        let profile: (String, String, String) = connection
            .query_row(
                "SELECT name, auth_mode, tls_mode FROM connection_profiles WHERE id = 'existing'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            profile,
            (
                "Existing profile".into(),
                "password".into(),
                "verify-full".into()
            )
        );
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
    }

    #[test]
    fn upgrades_a_development_database_with_already_added_v2_columns() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(INITIAL_MIGRATION).unwrap();
        connection
            .execute_batch(
                "ALTER TABLE connection_profiles ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'password';
                 ALTER TABLE connection_profiles ADD COLUMN tls_mode TEXT NOT NULL DEFAULT 'verify-full';
                 ALTER TABLE connection_profiles ADD COLUMN ssh_auth_mode TEXT NOT NULL DEFAULT 'password';",
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();

        migrate(&connection).unwrap();

        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, 2);
    }
}
