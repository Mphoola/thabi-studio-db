//! Secrets live in the operating system's credential store, through
//! `keyring-core`: Keychain on macOS, Credential Manager on Windows and the
//! Secret Service (GNOME Keyring, KWallet) on Linux. SQLite only ever holds
//! an opaque reference to them.

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
    let store = apple_native_keyring_store::keychain::Store::new();
    #[cfg(target_os = "windows")]
    let store = windows_native_keyring_store::Store::new();
    #[cfg(target_os = "linux")]
    let store = dbus_secret_service_keyring_store::Store::new();
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    keyring_core::set_default_store(store.map_err(|_| AppError::credential_access())?);
    Ok(())
}

pub struct OsCredentialStore;

fn entry(reference: &str) -> Result<Entry, AppError> {
    Entry::new(SERVICE_NAME, reference).map_err(|_| AppError::credential_access())
}

impl CredentialStore for OsCredentialStore {
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
