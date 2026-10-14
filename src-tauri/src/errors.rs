use serde::Serialize;
use std::collections::BTreeMap;

/// Error returned to the frontend as `{ kind, message, detail?, code?, fields? }`.
///
/// `message` is written for the user. `detail` carries the database server's
/// own message when there is one, already sanitised (see
/// `database::error::sanitise`). Secrets must never reach any field.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<BTreeMap<String, String>>,
}

impl AppError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            detail: None,
            code: None,
            fields: None,
        }
    }

    pub fn with_detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail.filter(|value| !value.is_empty());
        self
    }

    pub fn with_code(mut self, code: Option<String>) -> Self {
        self.code = code.filter(|value| !value.is_empty());
        self
    }

    pub fn validation(fields: BTreeMap<String, String>) -> Self {
        Self {
            fields: Some(fields),
            ..Self::new("validation", "Check the highlighted fields.")
        }
    }

    pub fn storage() -> Self {
        Self::new("storage", "Local data couldn't be accessed.")
    }

    pub fn background_task() -> Self {
        Self::new("internal", "Thabi Studio couldn't finish that request.")
    }

    pub fn credential_access() -> Self {
        Self::new(
            "credential_access",
            "Keychain couldn't be accessed. Check Keychain Access and try again.",
        )
    }

    pub fn credential_missing() -> Self {
        Self::new(
            "credential_missing",
            "A saved password is missing from Keychain. Enter it again to continue.",
        )
    }

    pub fn not_found() -> Self {
        Self::new("not_found", "That saved connection couldn't be found.")
    }

    pub fn unsupported_driver() -> Self {
        Self::new("unsupported_driver", "That database type isn't supported.")
    }

    pub fn unsupported_ssh() -> Self {
        Self::new(
            "ssh_unavailable",
            "SSH tunnels aren't available yet. Turn off SSH to connect directly.",
        )
    }

    pub fn not_connected() -> Self {
        Self::new("not_connected", "That connection isn't open.")
    }

    pub fn connection_in_progress() -> Self {
        Self::new(
            "connection_in_progress",
            "This connection is still opening. Try again in a moment.",
        )
    }

    pub fn connection_cancelled() -> Self {
        Self::new("cancelled", "Connecting was cancelled.")
    }

    pub fn connection_close_failed() -> Self {
        Self::new(
            "connection_close",
            "The database connection couldn't be closed cleanly.",
        )
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}
