//! Secrets live in the macOS Keychain (through `keyring-core` and its Apple
//! store). SQLite only ever holds an opaque reference to them.

use crate::errors::AppError;
use keyring_core::{Entry, Error};

const SERVICE_NAME: &str = "com.thabi.studio";

pub trait CredentialStore: Send + Sync {
    fn set(&self, reference: &str, value: &str) -> Result<(), AppError>;
    fn get(&self, reference: &str) -> Result<Option<String>, AppError>;
    fn delete(&self, reference: &str) -> Result<(), AppError>;
}

/// Registers the platform credential store. Call once at startup.
pub fn init_platform_store() -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    {
        let store = apple_native_keyring_store::keychain::Store::new()
            .map_err(|_| AppError::credential_access())?;
        keyring_core::set_default_store(store);
    }
    Ok(())
}

pub struct KeychainCredentialStore;

fn entry(reference: &str) -> Result<Entry, AppError> {
    Entry::new(SERVICE_NAME, reference).map_err(|_| AppError::credential_access())
}

impl CredentialStore for KeychainCredentialStore {
    fn set(&self, reference: &str, value: &str) -> Result<(), AppError> {
        entry(reference)?
            .set_password(value)
            .map_err(|_| AppError::credential_access())
    }

    fn get(&self, reference: &str) -> Result<Option<String>, AppError> {
        match entry(reference)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(Error::NoEntry) => Ok(None),
            Err(_) => Err(AppError::credential_access()),
        }
    }

    fn delete(&self, reference: &str) -> Result<(), AppError> {
        match entry(reference)?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(_) => Err(AppError::credential_access()),
        }
    }
}
