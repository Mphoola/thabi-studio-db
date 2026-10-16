//! The optional app password (see docs/decisions/0033-app-password.md).
//!
//! When it is set, Thabi Studio starts locked and can lock again after a
//! while idle. The lock is enforced here, not in the webview: while locked,
//! every command except the few the lock screen needs is refused before it
//! runs (`allowed_while_locked`), so no query, connection or saved password is
//! reachable until the password is entered.
//!
//! Only an Argon2id hash of the password is kept, in the OS credential store
//! next to the connection passwords it protects.

use crate::credentials::CredentialStore;
use crate::errors::AppError;
use argon2::Argon2;
use argon2::password_hash::{
    PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const MIN_LENGTH: usize = 10;
/// The credential-store entry holding the password hash.
const HASH_REFERENCE: &str = "app-password";
/// Wrong guesses allowed before each further guess has to wait.
const FREE_ATTEMPTS: u32 = 5;
const MAX_WAIT: Duration = Duration::from_secs(15 * 60);

/// Commands the lock screen itself needs. Everything else waits for unlock.
pub fn allowed_while_locked(command: &str) -> bool {
    matches!(
        command,
        "app_info"
            | "get_app_settings"
            | "import_legacy_preferences"
            | "app_lock_status"
            | "unlock_app"
            | "lock_app"
            | "reset_app_password"
            | "recovery_options"
            | "recover_with_code"
            | "recover_with_answers"
    ) || command.starts_with("plugin:")
}

/// What is wrong with a proposed password, or nothing.
pub fn strength_error(password: &str) -> Option<&'static str> {
    if password.chars().count() < MIN_LENGTH {
        Some("Use at least 10 characters.")
    } else if !password.chars().any(char::is_uppercase) {
        Some("Add an uppercase letter.")
    } else if !password.chars().any(char::is_lowercase) {
        Some("Add a lowercase letter.")
    } else {
        None
    }
}

pub fn hash(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::background_task())
}

pub fn verify(stored: &str, password: &str) -> bool {
    PasswordHash::new(stored)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

pub fn read_hash(credentials: &dyn CredentialStore) -> Result<Option<String>, AppError> {
    credentials.get(HASH_REFERENCE)
}

pub fn store_hash(credentials: &dyn CredentialStore, hash: &str) -> Result<(), AppError> {
    credentials.set(HASH_REFERENCE, hash)
}

pub fn delete_hash(credentials: &dyn CredentialStore) -> Result<(), AppError> {
    credentials.delete(HASH_REFERENCE)
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LockStatus {
    pub enabled: bool,
    pub locked: bool,
    /// Seconds before another guess is accepted; 0 when one can be made now.
    pub retry_after_seconds: u64,
}

#[derive(Default)]
struct Inner {
    /// Whether the credential store has been read. Until it has, the app
    /// counts as locked so nothing slips through at startup.
    loaded: bool,
    enabled: bool,
    locked: bool,
    failures: u32,
    blocked_until: Option<Instant>,
}

#[derive(Default)]
pub struct AppLock {
    inner: Mutex<Inner>,
}

impl AppLock {
    fn with<T>(&self, work: impl FnOnce(&mut Inner) -> T) -> T {
        // A poisoned lock still holds valid flags; keep enforcing them.
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        work(&mut inner)
    }

    pub fn is_locked(&self) -> bool {
        self.with(|inner| !inner.loaded || inner.locked)
    }

    pub fn is_loaded(&self) -> bool {
        self.with(|inner| inner.loaded)
    }

    /// Records what the credential store holds. The first load locks the app
    /// when a password is set; later loads keep the current state.
    pub fn load(&self, enabled: bool) {
        self.with(|inner| {
            if !inner.loaded {
                inner.locked = enabled;
            }
            inner.loaded = true;
            inner.enabled = enabled;
        });
    }

    pub fn status(&self) -> LockStatus {
        self.with(|inner| LockStatus {
            enabled: inner.enabled,
            locked: !inner.loaded || inner.locked,
            retry_after_seconds: inner
                .blocked_until
                .map(|until| until.saturating_duration_since(Instant::now()))
                .map(|wait| wait.as_secs() + u64::from(wait.subsec_nanos() > 0))
                .unwrap_or(0),
        })
    }

    pub fn lock(&self) {
        self.with(|inner| inner.locked = inner.enabled);
    }

    /// Sets whether a password exists and unlocks: the person who just set,
    /// removed or reset it is the one at the keyboard.
    pub fn set_enabled(&self, enabled: bool) {
        self.with(|inner| {
            inner.loaded = true;
            inner.enabled = enabled;
            inner.locked = false;
            inner.failures = 0;
            inner.blocked_until = None;
        });
    }

    /// Refuses a guess while an earlier run of wrong ones is being waited out.
    pub fn check_attempt(&self) -> Result<(), AppError> {
        let status = self.status();
        if status.retry_after_seconds > 0 {
            return Err(too_many_attempts(status.retry_after_seconds));
        }
        Ok(())
    }

    pub fn record(&self, correct: bool) {
        self.with(|inner| {
            if correct {
                inner.failures = 0;
                inner.blocked_until = None;
                return;
            }
            inner.failures += 1;
            if let Some(extra) = inner.failures.checked_sub(FREE_ATTEMPTS) {
                // 30 s after the fifth wrong guess, doubling up to 15 minutes.
                let wait = Duration::from_secs(30)
                    .saturating_mul(1 << extra.min(10))
                    .min(MAX_WAIT);
                inner.blocked_until = Some(Instant::now() + wait);
            }
        });
    }

    pub fn unlock(&self) {
        self.with(|inner| inner.locked = false);
    }
}

pub fn locked() -> AppError {
    AppError::new(
        "locked",
        "Thabi Studio is locked. Enter the app password to continue.",
    )
}

pub fn wrong_password(field: &str) -> AppError {
    AppError::validation(BTreeMap::from([(
        field.to_owned(),
        "That password isn't right.".to_owned(),
    )]))
}

fn too_many_attempts(seconds: u64) -> AppError {
    AppError::new(
        "too_many_attempts",
        "Too many wrong passwords. Wait a moment, then try again.",
    )
    .with_data(serde_json::json!({ "retryAfterSeconds": seconds }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_need_length_and_mixed_case() {
        assert!(strength_error("Short1").is_some());
        assert!(strength_error("alllowercase1").is_some());
        assert!(strength_error("ALLUPPERCASE1").is_some());
        assert_eq!(strength_error("Long enough Pw"), None);
    }

    #[test]
    fn hashes_verify_only_the_same_password() {
        let stored = hash("Correct Horse 1").unwrap();
        assert!(stored.starts_with("$argon2id$"));
        assert!(verify(&stored, "Correct Horse 1"));
        assert!(!verify(&stored, "correct horse 1"));
        assert!(!verify("not a hash", "Correct Horse 1"));
    }

    #[test]
    fn locked_until_loaded_and_then_while_a_password_is_set() {
        let lock = AppLock::default();
        assert!(lock.is_locked());
        lock.load(true);
        assert!(lock.is_locked());
        lock.unlock();
        assert!(!lock.is_locked());
        lock.load(true);
        assert!(!lock.is_locked(), "a reload keeps the unlocked state");
        lock.lock();
        assert!(lock.is_locked());

        let open = AppLock::default();
        open.load(false);
        open.lock();
        assert!(!open.is_locked(), "nothing to lock without a password");
    }

    #[test]
    fn repeated_wrong_guesses_have_to_wait() {
        let lock = AppLock::default();
        lock.load(true);
        for _ in 0..FREE_ATTEMPTS - 1 {
            lock.record(false);
            assert!(lock.check_attempt().is_ok());
        }
        lock.record(false);
        let error = lock.check_attempt().unwrap_err();
        assert_eq!(error.kind, "too_many_attempts");
        assert_eq!(lock.status().retry_after_seconds, 30);
        lock.set_enabled(true);
        assert!(lock.check_attempt().is_ok());
    }

    #[test]
    fn only_the_lock_screen_commands_pass_while_locked() {
        assert!(allowed_while_locked("unlock_app"));
        assert!(allowed_while_locked("plugin:event|listen"));
        assert!(!allowed_while_locked("run_statement"));
        assert!(!allowed_while_locked("list_connection_profiles"));
    }
}
