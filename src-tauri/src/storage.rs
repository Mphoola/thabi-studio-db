//! The app's own SQLite database. Migrations are numbered SQL files that run
//! once, in order, each inside a transaction. A migration file must never be
//! edited after it has shipped; add a new one instead.

use crate::errors::AppError;
use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/0001_local_profiles.sql"),
    include_str!("../migrations/0002_profile_security_settings.sql"),
    include_str!("../migrations/0003_tls_ca_path.sql"),
    include_str!("../migrations/0004_ssh_auth.sql"),
    include_str!("../migrations/0005_history_saved_tabs.sql"),
    include_str!("../migrations/0006_saved_query_folders.sql"),
    include_str!("../migrations/0007_safe_mode.sql"),
    include_str!("../migrations/0008_connection_groups.sql"),
    include_str!("../migrations/0009_ask_every_time.sql"),
    include_str!("../migrations/0010_startup_commands.sql"),
];

/// The migration files in order, for tests that need a schema from before the
/// latest one.
#[cfg(test)]
pub(crate) fn all_migrations() -> &'static [&'static str] {
    MIGRATIONS
}

pub fn migrate(connection: &mut Connection) -> Result<(), AppError> {
    let version: usize = connection
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .map_err(|_| AppError::storage())?
        .try_into()
        .map_err(|_| AppError::storage())?;
    if version > MIGRATIONS.len() {
        return Err(AppError::new(
            "storage",
            "This database was created by a newer version of Thabi Studio.",
        ));
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(version) {
        let transaction = connection.transaction().map_err(|_| AppError::storage())?;
        transaction
            .execute_batch(sql)
            .map_err(|_| AppError::storage())?;
        transaction
            .pragma_update(None, "user_version", (index + 1) as i64)
            .map_err(|_| AppError::storage())?;
        transaction.commit().map_err(|_| AppError::storage())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user_version(connection: &Connection) -> i64 {
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn creates_the_current_schema() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();

        assert_eq!(user_version(&connection), MIGRATIONS.len() as i64);
        let settings: i64 = connection
            .query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(settings, 7);
    }

    #[test]
    fn running_twice_changes_nothing() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        migrate(&mut connection).unwrap();
        assert_eq!(user_version(&connection), MIGRATIONS.len() as i64);
    }

    #[test]
    fn upgrades_an_older_database_without_losing_profiles() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(MIGRATIONS[0]).unwrap();
        connection
            .execute(
                "INSERT INTO connection_profiles (id, name, environment, driver, host, port, username)
                 VALUES ('existing', 'Existing profile', 'local', 'postgres', 'localhost', 5432, 'user')",
                [],
            )
            .unwrap();
        connection.pragma_update(None, "user_version", 1).unwrap();

        migrate(&mut connection).unwrap();

        let profile: (String, String, String) = connection
            .query_row(
                "SELECT name, tls_mode, tls_ca_path FROM connection_profiles WHERE id = 'existing'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            profile,
            (
                "Existing profile".into(),
                "verify-full".into(),
                String::new()
            )
        );
    }

    #[test]
    fn refuses_a_database_from_a_newer_version() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
        assert!(migrate(&mut connection).is_err());
    }
}
