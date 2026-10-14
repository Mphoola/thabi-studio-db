use crate::errors::AppError;

const SERVICE_NAME: &str = "com.thabi.studio";

pub trait CredentialStore: Send + Sync {
    fn set(&self, reference: &str, value: &str) -> Result<(), AppError>;
    fn get(&self, reference: &str) -> Result<Option<String>, AppError>;
    fn delete(&self, reference: &str) -> Result<(), AppError>;
}

pub struct KeychainCredentialStore;

impl CredentialStore for KeychainCredentialStore {
    fn set(&self, reference: &str, value: &str) -> Result<(), AppError> {
        keyring::Entry::new(SERVICE_NAME, reference)
            .map_err(|_| AppError::credential_access())?
            .set_password(value)
            .map_err(|_| AppError::credential_access())
    }

    fn get(&self, reference: &str) -> Result<Option<String>, AppError> {
        match keyring::Entry::new(SERVICE_NAME, reference)
            .map_err(|_| AppError::credential_access())?
            .get_password()
        {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(AppError::credential_access()),
        }
    }

    fn delete(&self, reference: &str) -> Result<(), AppError> {
        match keyring::Entry::new(SERVICE_NAME, reference)
            .map_err(|_| AppError::credential_access())?
            .delete_credential()
        {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(AppError::credential_access()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MemoryCredentialStore;

    impl CredentialStore for MemoryCredentialStore {
        fn set(&self, _reference: &str, _value: &str) -> Result<(), AppError> {
            Ok(())
        }

        fn get(&self, _reference: &str) -> Result<Option<String>, AppError> {
            Ok(None)
        }

        fn delete(&self, _reference: &str) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[test]
    fn secret_values_are_never_part_of_profile_errors() {
        let error = AppError::credential_access();
        assert!(!error.message.contains("password"));
        assert!(
            MemoryCredentialStore
                .get("opaque-reference")
                .unwrap()
                .is_none()
        );
    }
}
