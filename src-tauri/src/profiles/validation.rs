use crate::errors::AppError;
use std::{collections::BTreeMap, path::Path};

/// Profile input after validation and normalisation.
pub struct ValidatedProfile {
    pub name: String,
    pub environment: String,
    pub driver: String,
    pub host: String,
    pub port: u16,
    pub database_name: String,
    pub username: String,
    pub tls_mode: String,
    pub tls_ca_path: String,
    pub ssh_enabled: bool,
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_username: String,
    pub ssh_auth_mode: String,
    pub ssh_key_path: String,
    pub database_password: Option<String>,
    pub ssh_password: Option<String>,
    pub ssh_key_passphrase: Option<String>,
    pub clear_database_password: bool,
    pub clear_ssh_password: bool,
    pub clear_ssh_key_passphrase: bool,
    pub safe_mode: String,
    pub group_name: String,
    pub colour: String,
}

/// Tag tokens a connection may be coloured with (`tag-*` in tokens.css).
pub const COLOURS: [&str; 8] = [
    "red", "orange", "yellow", "green", "teal", "blue", "purple", "gray",
];

/// Largest CA certificate file accepted (a bundle is a few hundred KB at most).
const MAX_CA_FILE_BYTES: u64 = 1024 * 1024;
/// Largest private key file accepted (OpenSSH keys are a few KB).
const MAX_KEY_FILE_BYTES: u64 = 100 * 1024;

pub fn validate(input: super::ProfileInput) -> Result<ValidatedProfile, AppError> {
    let mut fields = BTreeMap::new();

    let name = input.name.trim().to_owned();
    if name.is_empty() {
        fields.insert("name".into(), "Enter a connection name.".into());
    } else if name.chars().count() > 80 {
        fields.insert("name".into(), "Use 80 characters or fewer.".into());
    }

    if !matches!(
        input.environment.as_str(),
        "local" | "development" | "testing" | "staging" | "production"
    ) {
        fields.insert("environment".into(), "Choose an environment.".into());
    }
    if !matches!(input.driver.as_str(), "mysql" | "postgres" | "sqlserver") {
        fields.insert("driver".into(), "Choose a supported database.".into());
    }

    let host = match normalise_host(&input.host) {
        Ok(host) => host,
        Err(message) => {
            fields.insert("host".into(), message.into());
            String::new()
        }
    };
    let port = validate_port(&input.port, "port", &mut fields);

    let database_name = input.database_name.trim().to_owned();
    if database_name.chars().count() > 128 || database_name.chars().any(char::is_control) {
        fields.insert("databaseName".into(), "Use a valid database name.".into());
    }

    let username = input.username.trim().to_owned();
    if username.is_empty() {
        fields.insert("username".into(), "Enter a user name.".into());
    } else if username.chars().count() > 128 || username.chars().any(char::is_control) {
        fields.insert("username".into(), "Use 128 characters or fewer.".into());
    }

    let tls_mode = input.tls_mode.trim().to_owned();
    if !matches!(tls_mode.as_str(), "verify-full" | "require" | "disable") {
        fields.insert("tlsMode".into(), "Choose a TLS setting.".into());
    }
    let tls_ca_path = if tls_mode == "verify-full" {
        let path = input.tls_ca_path.trim().to_owned();
        if let Err(message) = check_ca_file(&path) {
            fields.insert("tlsCaPath".into(), message.into());
        }
        path
    } else {
        String::new()
    };

    let mut ssh_host = input.ssh_host.trim().to_owned();
    let ssh_username = input.ssh_username.trim().to_owned();
    let ssh_port = if input.ssh_enabled {
        match normalise_host(&ssh_host) {
            Ok(host) => ssh_host = host,
            Err(_) => {
                fields.insert(
                    "sshHost".into(),
                    "Enter a valid SSH host name or IP address.".into(),
                );
            }
        }
        if ssh_username.is_empty() {
            fields.insert("sshUsername".into(), "Enter an SSH user name.".into());
        } else if ssh_username.chars().count() > 128 || ssh_username.chars().any(char::is_control) {
            fields.insert("sshUsername".into(), "Use 128 characters or fewer.".into());
        }
        validate_port(&input.ssh_port, "sshPort", &mut fields)
    } else {
        22
    };
    let ssh_auth_mode = input.ssh_auth_mode.trim().to_owned();
    if !matches!(ssh_auth_mode.as_str(), "password" | "key" | "agent") {
        fields.insert("sshAuthMode".into(), "Choose how to sign in to SSH.".into());
    }
    let safe_mode = input.safe_mode.trim().to_owned();
    if !matches!(
        safe_mode.as_str(),
        "off" | "confirm-writes" | "confirm-everything" | "read-only"
    ) {
        fields.insert("safeMode".into(), "Choose a safe mode.".into());
    }

    // The group name is shown as a heading and compared case-insensitively,
    // so it only needs to be short, single-line and free of control characters.
    let group_name = input.group_name.trim().to_owned();
    if group_name.chars().count() > 60 || group_name.chars().any(char::is_control) {
        fields.insert(
            "groupName".into(),
            "Use a group name of 60 characters or fewer.".into(),
        );
    }
    let colour = input.colour.trim().to_owned();
    if !colour.is_empty() && !COLOURS.contains(&colour.as_str()) {
        fields.insert(
            "colour".into(),
            "Choose one of the available colours.".into(),
        );
    }
    let ssh_key_path = if ssh_auth_mode == "key" {
        let path = input.ssh_key_path.trim().to_owned();
        if input.ssh_enabled
            && let Err(message) = check_key_file(&path)
        {
            fields.insert("sshKeyPath".into(), message.into());
        }
        path
    } else {
        String::new()
    };

    for (field, secret) in [
        ("databasePassword", input.database_password.as_ref()),
        ("sshPassword", input.ssh_password.as_ref()),
        ("sshKeyPassphrase", input.ssh_key_passphrase.as_ref()),
    ] {
        if secret.is_some_and(|value| value.chars().count() > 4096) {
            fields.insert(field.into(), "Use 4,096 characters or fewer.".into());
        }
    }
    if input.clear_database_password
        && input
            .database_password
            .as_deref()
            .is_some_and(|s| !s.is_empty())
    {
        fields.insert(
            "databasePassword".into(),
            "Enter a password or choose to remove the saved password, not both.".into(),
        );
    }
    if input.clear_ssh_password && input.ssh_password.as_deref().is_some_and(|s| !s.is_empty()) {
        fields.insert(
            "sshPassword".into(),
            "Enter a password or choose to remove the saved password, not both.".into(),
        );
    }

    if input.clear_ssh_key_passphrase
        && input
            .ssh_key_passphrase
            .as_deref()
            .is_some_and(|s| !s.is_empty())
    {
        fields.insert(
            "sshKeyPassphrase".into(),
            "Enter a passphrase or choose to remove the saved one, not both.".into(),
        );
    }

    if !fields.is_empty() {
        return Err(AppError::validation(fields));
    }

    Ok(ValidatedProfile {
        name,
        environment: input.environment,
        driver: input.driver,
        host,
        port,
        database_name,
        username,
        tls_mode,
        tls_ca_path,
        ssh_enabled: input.ssh_enabled,
        ssh_host,
        ssh_port,
        ssh_username,
        ssh_auth_mode,
        ssh_key_path,
        database_password: input.database_password.filter(|value| !value.is_empty()),
        ssh_password: input.ssh_password.filter(|value| !value.is_empty()),
        ssh_key_passphrase: input.ssh_key_passphrase.filter(|value| !value.is_empty()),
        clear_database_password: input.clear_database_password,
        clear_ssh_password: input.clear_ssh_password,
        clear_ssh_key_passphrase: input.clear_ssh_key_passphrase,
        safe_mode,
        group_name,
        colour,
    })
}

fn validate_port(value: &str, field: &str, fields: &mut BTreeMap<String, String>) -> u16 {
    match value.parse::<u16>() {
        Ok(port) if port > 0 => port,
        _ => {
            fields.insert(field.into(), "Enter a port from 1 to 65,535.".into());
            0
        }
    }
}

fn check_key_file(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("Choose your SSH private key file.");
    }
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("Choose the SSH key file again.");
    }
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() && meta.len() <= MAX_KEY_FILE_BYTES => Ok(()),
        Ok(meta) if meta.is_file() => Err("That file is too large to be an SSH key."),
        _ => Err("That SSH key file can't be found."),
    }
}

/// Empty means "use the system's trusted certificates".
fn check_ca_file(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Ok(());
    }
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("Choose the CA certificate file again.");
    }
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() && meta.len() <= MAX_CA_FILE_BYTES => Ok(()),
        Ok(meta) if meta.is_file() => Err("That file is too large to be a CA certificate."),
        _ => Err("That CA certificate file can't be found."),
    }
}

/// Trims and checks a host. Accepts DNS names (including underscores, as
/// Docker Compose service names use them) and IP addresses; brackets around
/// an IPv6 address are removed. Returns a field message on failure.
pub fn normalise_host(host: &str) -> Result<String, &'static str> {
    let host = host.trim();
    if host.contains('\\') {
        return Err(
            "Named instances (host\\instance) aren't supported. Enter the host and the instance's port.",
        );
    }
    if host.is_empty() || host.chars().count() > 253 || host.chars().any(char::is_whitespace) {
        return Err("Enter a valid host name or IP address.");
    }
    let unwrapped = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host);
    if unwrapped.parse::<std::net::IpAddr>().is_ok() {
        return Ok(unwrapped.to_owned());
    }
    let looks_like_ipv4 = unwrapped
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
        && unwrapped.matches('.').count() == 3;
    let domain = unwrapped.strip_suffix('.').unwrap_or(unwrapped);
    let valid = !looks_like_ipv4
        && !domain.is_empty()
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                && label
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                && label
                    .chars()
                    .last()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    if valid {
        Ok(unwrapped.to_owned())
    } else {
        Err("Enter a valid host name or IP address.")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::profiles::ProfileInput;

    pub fn input() -> ProfileInput {
        ProfileInput {
            name: "Local database".into(),
            environment: "local".into(),
            driver: "postgres".into(),
            host: "localhost".into(),
            port: "5432".into(),
            database_name: "demo".into(),
            username: "user".into(),
            tls_mode: "disable".into(),
            tls_ca_path: String::new(),
            ssh_enabled: false,
            ssh_host: String::new(),
            ssh_port: "22".into(),
            ssh_username: String::new(),
            ssh_auth_mode: "password".into(),
            ssh_key_path: String::new(),
            database_password: None,
            ssh_password: None,
            clear_database_password: false,
            clear_ssh_password: false,
            ssh_key_passphrase: None,
            clear_ssh_key_passphrase: false,
            safe_mode: "off".into(),
            group_name: String::new(),
            colour: String::new(),
        }
    }

    #[test]
    fn validates_and_normalises_hosts() {
        assert_eq!(
            normalise_host(" db.example.com ").unwrap(),
            "db.example.com"
        );
        assert_eq!(normalise_host("127.0.0.1").unwrap(), "127.0.0.1");
        assert_eq!(normalise_host("::1").unwrap(), "::1");
        assert_eq!(normalise_host("[::1]").unwrap(), "::1");
        assert_eq!(normalise_host("my_db").unwrap(), "my_db");
        assert!(normalise_host("-invalid.example").is_err());
        assert!(normalise_host("host name").is_err());
        assert!(normalise_host("999.1.1.1").is_err());
        assert!(
            normalise_host("sql01\\prod")
                .unwrap_err()
                .contains("Named instances")
        );
    }

    #[test]
    fn reports_invalid_ports_and_required_fields_by_field() {
        let mut invalid = input();
        invalid.name = " ".into();
        invalid.port = "65536".into();
        invalid.host = "bad host".into();
        invalid.tls_mode = "prefer".into();

        let error = validate(invalid)
            .err()
            .expect("invalid profile should fail");

        assert_eq!(error.kind, "validation");
        let fields = error.fields.unwrap();
        for field in ["name", "port", "host", "tlsMode"] {
            assert!(fields.contains_key(field), "missing {field}");
        }
    }

    #[test]
    fn accepts_valid_profile_without_secrets_or_ssh() {
        let validated = validate(input()).unwrap();
        assert_eq!(validated.driver, "postgres");
        assert_eq!(validated.port, 5432);
        assert_eq!(validated.tls_mode, "disable");
        assert!(validated.database_password.is_none());
        assert!(!validated.ssh_enabled);
    }

    #[test]
    fn key_authentication_needs_a_readable_key_file() {
        let mut with_key = input();
        with_key.ssh_enabled = true;
        with_key.ssh_host = "bastion.example.com".into();
        with_key.ssh_username = "deploy".into();
        with_key.ssh_auth_mode = "key".into();
        let fields = validate(with_key.clone()).err().unwrap().fields.unwrap();
        assert_eq!(fields["sshKeyPath"], "Choose your SSH private key file.");

        with_key.ssh_auth_mode = "agent".into();
        with_key.ssh_key_path = "/ignored".into();
        assert_eq!(validate(with_key).unwrap().ssh_key_path, "");
    }

    #[test]
    fn ca_file_must_exist_and_only_applies_to_verification() {
        let mut with_ca = input();
        with_ca.tls_mode = "verify-full".into();
        with_ca.tls_ca_path = "/definitely/not/here.pem".into();
        let fields = validate(with_ca.clone()).err().unwrap().fields.unwrap();
        assert!(fields.contains_key("tlsCaPath"));

        with_ca.tls_mode = "require".into();
        assert_eq!(validate(with_ca).unwrap().tls_ca_path, "");
    }
}
