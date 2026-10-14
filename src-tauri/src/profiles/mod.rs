//! Saved connection profiles: metadata in SQLite, secrets in Keychain.

mod repository;
mod validation;

use serde::{Deserialize, Serialize};

pub use repository::{
    connect_params, connect_params_for_input, create_profile, delete_profile, duplicate_profile,
    list_profiles, mark_connected, select_profile, update_validated_profile,
};
pub use validation::{ValidatedProfile, validate};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: String,
    pub name: String,
    pub environment: String,
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
    pub username: String,
    pub auth_mode: String,
    /// "verify-full", "require" or "disable". See `database::TlsMode`.
    pub tls_mode: String,
    /// Custom CA certificate used with "verify-full"; empty for system roots.
    pub tls_ca_path: String,
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub ssh_auth_mode: String,
    pub has_database_password: bool,
    pub has_ssh_password: bool,
    pub created_at: String,
    pub updated_at: String,
    pub last_connected_at: Option<String>,
}

fn default_tls_mode() -> String {
    "verify-full".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    pub environment: String,
    pub driver: String,
    pub host: String,
    pub port: String,
    pub database_name: String,
    pub username: String,
    #[serde(default = "default_tls_mode")]
    pub tls_mode: String,
    #[serde(default)]
    pub tls_ca_path: String,
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_username: String,
    #[serde(default)]
    pub database_password: Option<String>,
    #[serde(default)]
    pub ssh_password: Option<String>,
    #[serde(default)]
    pub clear_database_password: bool,
    #[serde(default)]
    pub clear_ssh_password: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileWriteResult {
    pub profile: ConnectionProfile,
    pub warning: Option<String>,
}
