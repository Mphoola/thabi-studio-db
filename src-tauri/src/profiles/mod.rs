//! Saved connection profiles: metadata in SQLite, secrets in the OS credential store.

mod repository;
mod validation;

use serde::{Deserialize, Serialize};

pub use repository::{
    apply_one_off_secrets, connect_params, connect_params_for_input, create_profile,
    delete_profile, duplicate_profile, list_profiles, mark_connected, reorder_profiles,
    required_secret, select_profile, update_validated_profile,
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
    /// Client certificate and key for mutual TLS, as file paths.
    pub tls_client_cert: String,
    pub tls_client_key: String,
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    /// "password", "key" or "agent".
    pub ssh_auth_mode: String,
    /// Private key file for "key" authentication. The file is read at
    /// connect time and never copied.
    pub ssh_key_path: String,
    pub has_database_password: bool,
    pub has_ssh_password: bool,
    pub has_ssh_key_passphrase: bool,
    /// "off", "confirm-writes", "confirm-everything" or "read-only".
    pub safe_mode: String,
    /// User-named group, or "" to stay in its environment group.
    pub group_name: String,
    /// Tag token name for a custom colour, or "" to use the environment's.
    pub colour: String,
    /// Manual order inside the group; ties fall back to the name.
    pub sort_order: i64,
    /// Secrets the user types at connect time instead of keeping saved.
    pub ask_database_password: bool,
    pub ask_ssh_password: bool,
    pub ask_ssh_key_passphrase: bool,
    /// Keep-alive interval for this connection's SSH session; 0 is the default.
    pub ssh_keepalive_seconds: u32,
    /// Statements run on every new session, one per line.
    pub startup_commands: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_connected_at: Option<String>,
}

fn default_tls_mode() -> String {
    "verify-full".into()
}

fn default_safe_mode() -> String {
    "off".into()
}

fn default_ssh_auth_mode() -> String {
    "password".into()
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
    #[serde(default)]
    pub tls_client_cert: String,
    #[serde(default)]
    pub tls_client_key: String,
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_username: String,
    #[serde(default = "default_ssh_auth_mode")]
    pub ssh_auth_mode: String,
    #[serde(default)]
    pub ssh_key_path: String,
    #[serde(default)]
    pub database_password: Option<String>,
    #[serde(default)]
    pub ssh_password: Option<String>,
    #[serde(default)]
    pub clear_database_password: bool,
    #[serde(default)]
    pub clear_ssh_password: bool,
    #[serde(default)]
    pub ssh_key_passphrase: Option<String>,
    #[serde(default)]
    pub clear_ssh_key_passphrase: bool,
    #[serde(default = "default_safe_mode")]
    pub safe_mode: String,
    #[serde(default)]
    pub group_name: String,
    #[serde(default)]
    pub colour: String,
    #[serde(default)]
    pub ask_database_password: bool,
    #[serde(default)]
    pub ask_ssh_password: bool,
    #[serde(default)]
    pub ask_ssh_key_passphrase: bool,
    /// Keep-alive interval in seconds; 0 uses the default.
    #[serde(default)]
    pub ssh_keepalive_seconds: String,
    #[serde(default)]
    pub startup_commands: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileWriteResult {
    pub profile: ConnectionProfile,
    pub warning: Option<String>,
}
