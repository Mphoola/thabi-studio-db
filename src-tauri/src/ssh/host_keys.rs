//! Trusted SSH host keys.
//!
//! Keys the user accepted in Thabi Studio are stored in SQLite. Keys in the
//! user's `~/.ssh/known_hosts` are trusted too (read-only), so servers they
//! already use from Terminal don't prompt again. An unknown key always
//! needs the user's confirmation; a changed key is always refused.

use crate::errors::AppError;
use rusqlite::{Connection, params};
use russh::keys::{HashAlg, PublicKey};

#[derive(Clone, Debug)]
pub struct KnownHost {
    pub key: PublicKey,
    /// True when the key came from ~/.ssh/known_hosts.
    pub from_openssh: bool,
}

/// ~/.ssh/known_hosts (%USERPROFILE%\.ssh\known_hosts on Windows).
fn openssh_known_hosts() -> Option<std::path::PathBuf> {
    std::env::home_dir().map(|home| home.join(".ssh").join("known_hosts"))
}

pub fn known_host_keys(
    connection: &Connection,
    host: &str,
    port: u16,
) -> Result<Vec<KnownHost>, AppError> {
    let mut statement = connection
        .prepare("SELECT public_key FROM ssh_known_hosts WHERE host = ?1 AND port = ?2")
        .map_err(|_| AppError::storage())?;
    let mut keys: Vec<KnownHost> = statement
        .query_map(params![host, port], |row| row.get::<_, String>(0))
        .map_err(|_| AppError::storage())?
        .filter_map(Result::ok)
        .filter_map(|text| PublicKey::from_openssh(&text).ok())
        .map(|key| KnownHost {
            key,
            from_openssh: false,
        })
        .collect();
    if let Some(path) = openssh_known_hosts().filter(|p| p.exists())
        && let Ok(found) = russh::keys::known_hosts::known_host_keys_path(host, port, path)
    {
        keys.extend(found.into_iter().map(|(_, key)| KnownHost {
            key,
            from_openssh: true,
        }));
    }
    Ok(keys)
}

/// Stores a key the user confirmed. Replaces an earlier key of the same type.
pub fn trust_host_key(
    connection: &Connection,
    host: &str,
    port: u16,
    public_key: &str,
) -> Result<String, AppError> {
    let key = PublicKey::from_openssh(public_key.trim())
        .map_err(|_| AppError::new("validation", "That host key couldn't be read."))?;
    let algorithm = key.algorithm().to_string();
    let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
    let text = key.to_openssh().map_err(|_| AppError::background_task())?;
    connection
        .execute(
            "DELETE FROM ssh_known_hosts WHERE host = ?1 AND port = ?2 AND algorithm = ?3",
            params![host, port, algorithm],
        )
        .map_err(|_| AppError::storage())?;
    connection
        .execute(
            "INSERT INTO ssh_known_hosts (host, port, algorithm, public_key, fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![host, port, algorithm, text, fingerprint],
        )
        .map_err(|_| AppError::storage())?;
    Ok(fingerprint)
}

/// Forgets Thabi Studio's saved keys for a server. Returns true when
/// ~/.ssh/known_hosts still lists a key for it, which only the user can
/// remove (`ssh-keygen -R`).
pub fn forget_host_keys(connection: &Connection, host: &str, port: u16) -> Result<bool, AppError> {
    connection
        .execute(
            "DELETE FROM ssh_known_hosts WHERE host = ?1 AND port = ?2",
            params![host, port],
        )
        .map_err(|_| AppError::storage())?;
    Ok(known_host_keys(connection, host, port)?
        .iter()
        .any(|k| k.from_openssh))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrate;

    const KEY: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDWWQIbqI0abxnfVrE8JJDq61VupxJFl/o5opba9M1fu test";
    const OTHER: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFUCl4YJ7IyIa2yEUkDjnPsOJeBcMukxWYMKAuDcbw27 other";

    #[test]
    fn trusting_stores_one_key_per_type_and_forgetting_removes_it() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        // Use a host that can't be in the developer's own known_hosts.
        let host = "thabi-test.invalid";

        let fingerprint = trust_host_key(&connection, host, 22, KEY).unwrap();
        assert!(fingerprint.starts_with("SHA256:"));
        trust_host_key(&connection, host, 22, OTHER).unwrap();
        let keys = known_host_keys(&connection, host, 22).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].key, PublicKey::from_openssh(OTHER).unwrap());

        assert!(known_host_keys(&connection, host, 2222).unwrap().is_empty());
        assert!(!forget_host_keys(&connection, host, 22).unwrap());
        assert!(known_host_keys(&connection, host, 22).unwrap().is_empty());
    }

    #[test]
    fn rejects_text_that_is_not_a_public_key() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        assert_eq!(
            trust_host_key(&connection, "h", 22, "not a key")
                .unwrap_err()
                .kind,
            "validation"
        );
    }
}
