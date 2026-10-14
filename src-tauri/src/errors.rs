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
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

impl std::error::Error for AppError {}
