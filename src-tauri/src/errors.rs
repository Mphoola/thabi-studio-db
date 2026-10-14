use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: &'static str,
    pub message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<BTreeMap<String, String>>,
}

impl AppError {
    pub fn new(kind: &'static str, message: &'static str) -> Self {
        Self {
            kind,
            message,
            fields: None,
        }
    }

    pub fn validation(fields: BTreeMap<String, String>) -> Self {
        Self {
            kind: "validation",
            message: "Check the highlighted fields.",
            fields: Some(fields),
        }
    }

    pub fn storage() -> Self {
        Self::new("storage", "Local data couldn't be accessed.")
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

    pub fn unsupported_driver() -> Self {
        Self::new("unsupported_driver", "That database type isn't supported.")
    }

    pub fn unsupported_tls_mode() -> Self {
        Self::new(
            "tls_configuration",
            "This profile uses a TLS mode that isn't supported yet.",
        )
    }

    pub fn unsupported_ssh() -> Self {
        Self::new(
            "ssh_unavailable",
            "SSH-tunneled connections aren't available yet. Direct connections only.",
        )
    }

    pub fn connection_failed() -> Self {
        Self::new(
            "connection",
            "Couldn't connect. Check the host, port, credentials, and database name.",
        )
    }

    pub fn authentication_failed() -> Self {
        Self::new(
            "authentication",
            "The database rejected the login or selected database.",
        )
    }

    pub fn connection_timeout() -> Self {
        Self::new(
            "timeout",
            "The database connection timed out. Check the host and port.",
        )
    }

    pub fn tls_verification_failed() -> Self {
        Self::new(
            "tls",
            "The database TLS connection couldn't be verified. Check its certificate and host name.",
        )
    }

    pub fn connection_in_progress() -> Self {
        Self::new(
            "connection_in_progress",
            "This connection is still opening. Try again in a moment.",
        )
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
        formatter.write_str(self.message)
    }
}

impl std::error::Error for AppError {}
