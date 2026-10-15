//! Reading `~/.ssh/config`.
//!
//! Enough of the format to let someone type a `Host` alias where Thabi
//! Studio asks for an SSH host and have the same settings OpenSSH would use:
//! HostName, User, Port, IdentityFile and one ProxyJump hop. The file is only
//! ever read, never written.
//!
//! Deliberately not implemented, and said out loud in the UI when it applies:
//!
//! - Wildcards and negated patterns (`*`, `?`, `!`) are ignored. Only an alias
//!   that matches exactly resolves, so a broad pattern can't quietly change
//!   how one connection connects.
//! - `Include` is not followed. An include can pull in files the user didn't
//!   mean for this app to read, and deciding which are safe is guesswork.
//! - Match `ssh_config` semantics where the first value for a keyword wins,
//!   so a later, broader block cannot override an earlier specific one.

use std::path::PathBuf;

/// What one alias resolves to. `None` means "not set in the config".
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHostConfig {
    /// The alias as it was typed.
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    /// Absolute path to the identity file, with `~` expanded.
    pub identity_file: Option<String>,
    /// One jump host alias, as written. Empty means a direct connection.
    pub proxy_jump: String,
}

/// Where the user's config lives, if the home directory can be found.
pub fn user_config_path() -> Option<PathBuf> {
    std::env::home_dir().map(|home| home.join(".ssh").join("config"))
}

/// Resolves `alias` against the user's own `~/.ssh/config`, if it is readable.
pub fn resolve_user_alias(alias: &str) -> Option<SshHostConfig> {
    let path = user_config_path()?;
    let text = std::fs::read_to_string(path).ok()?;
    resolve(&text, alias)
}

/// The path shown to the user, for saying where the settings came from.
pub fn user_config_label() -> String {
    user_config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "~/.ssh/config".into())
}

/// A `ProxyJump` value may be `user@host:port`; this pulls out the host.
pub fn jump_target(value: &str) -> (Option<String>, String, Option<u16>) {
    let (user, rest) = match value.rsplit_once('@') {
        Some((user, rest)) => (Some(unquote(user)), rest),
        None => (None, value),
    };
    let (host, port) = match rest.rsplit_once(':') {
        // IPv6 literals keep their brackets and have no port here.
        Some((host, port)) if !host.ends_with(']') => match port.parse::<u16>() {
            Ok(port) => (host.to_owned(), Some(port)),
            Err(_) => (rest.to_owned(), None),
        },
        _ => (rest.to_owned(), None),
    };
    (user, unquote(&host), port)
}

fn unquote(value: &str) -> String {
    let trimmed = value.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or(trimmed)
        .to_owned()
}

/// Expands a leading `~` so identity files resolve like they do in OpenSSH.
fn expand_home(value: &str, home: Option<&PathBuf>) -> String {
    let path = unquote(value);
    if let (Some(home), Some(rest)) = (&home, path.strip_prefix("~/")) {
        return home.join(rest).display().to_string();
    }
    path
}

/// A block's patterns; a block applies only if one of them is the alias.
fn matches(alias: &str, patterns: &str) -> bool {
    patterns.split_whitespace().any(|pattern| {
        // Exact, case-insensitive, and never a wildcard or a negation.
        !pattern.contains(['*', '?', '!']) && pattern.eq_ignore_ascii_case(alias)
    })
}

/// Resolves `alias` in an OpenSSH config's text.
///
/// `home` is only used to expand `~` in IdentityFile; pass the user's home in
/// tests so they don't depend on the machine.
pub fn resolve_with_home(text: &str, alias: &str, home: Option<&PathBuf>) -> Option<SshHostConfig> {
    let alias = alias.trim();
    if alias.is_empty() {
        return None;
    }
    let mut found: Option<SshHostConfig> = None;
    let mut applies = false;
    // ssh_config: the first value seen for a keyword wins.
    let set = |config: &mut SshHostConfig, keyword: &str, value: &str| match keyword {
        "hostname" if config.hostname.is_none() => config.hostname = Some(unquote(value)),
        "user" if config.user.is_none() => config.user = Some(unquote(value)),
        "port" if config.port.is_none() => config.port = value.trim().parse().ok(),
        "identityfile" if config.identity_file.is_none() => {
            config.identity_file = Some(expand_home(value, home))
        }
        "proxyjump" if config.proxy_jump.is_empty() => {
            let (_, host, _) = jump_target(value);
            config.proxy_jump = host;
        }
        _ => {}
    };

    for line in text.lines() {
        let line = line.trim();
        // Comments run to the end of the line.
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (keyword, value) = match line.split_once('=') {
            // Both `Keyword value` and `Keyword=value` are valid.
            Some((keyword, value)) => (keyword.trim(), value.trim()),
            None => match line.split_once(char::is_whitespace) {
                Some((keyword, value)) => (keyword.trim(), value.trim()),
                None => continue,
            },
        };
        let keyword = keyword.to_lowercase();

        if keyword == "host" {
            if found.is_some() {
                break; // Past the end of every block that applies.
            }
            applies = matches(alias, value);
            if applies {
                found = Some(SshHostConfig {
                    alias: alias.to_owned(),
                    ..SshHostConfig::default()
                });
            }
            continue;
        }
        if !applies {
            continue;
        }
        let Some(config) = found.as_mut() else {
            continue;
        };
        // `Include` is skipped, not followed: see the module comment.
        set(config, &keyword, value);
    }

    found.filter(|config| {
        config.hostname.is_some()
            || config.user.is_some()
            || config.port.is_some()
            || config.identity_file.is_some()
            || !config.proxy_jump.is_empty()
    })
}

/// Resolves `alias` without a home directory for `~` expansion.
pub fn resolve(text: &str, alias: &str) -> Option<SshHostConfig> {
    resolve_with_home(text, alias, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
# Personal machines
Host laptop
    HostName laptop.example.com
    User deploy
    Port 2222
    IdentityFile ~/.ssh/id_ed25519

Host "quoted alias" spaced
    HostName space.example.com

Host *.example.net work-*
    HostName wildcard.example.net

Host work
    HostName work.internal
    User "second name"
    Include ~/.ssh/other.conf

Host jump
    HostName bastion.example.com
    User jumpuser
    ProxyJump other-jump:2222
"#;

    #[test]
    fn resolves_an_exact_alias() {
        let config = resolve(SAMPLE, "laptop").unwrap();
        assert_eq!(config.hostname.as_deref(), Some("laptop.example.com"));
        assert_eq!(config.user.as_deref(), Some("deploy"));
        assert_eq!(config.port, Some(2222));
        assert_eq!(config.identity_file.as_deref(), Some("~/.ssh/id_ed25519"));
        assert_eq!(config.proxy_jump, "");
    }

    #[test]
    fn keeps_quoted_values_but_takes_the_first_block() {
        let config = resolve(SAMPLE, "work").unwrap();
        assert_eq!(config.hostname.as_deref(), Some("work.internal"));
        // The quoted value keeps its spaces.
        assert_eq!(config.user.as_deref(), Some("second name"));
        // Include is not followed, so this host gets nothing from it.
        assert_eq!(config.identity_file, None);
    }

    #[test]
    fn ignores_wildcard_and_negated_patterns() {
        assert_eq!(resolve(SAMPLE, "anything"), None);
        assert_eq!(resolve(SAMPLE, "work-db"), None);
        assert_eq!(resolve(SAMPLE, "*.example.net"), None);
    }

    #[test]
    fn matches_names_case_insensitively_and_reads_a_jump_host() {
        let config = resolve(SAMPLE, "LAPTOP").unwrap();
        assert_eq!(config.hostname.as_deref(), Some("laptop.example.com"));

        let jump = resolve(SAMPLE, "jump").unwrap();
        assert_eq!(jump.proxy_jump, "other-jump");
        assert_eq!(jump.user.as_deref(), Some("jumpuser"));
    }

    #[test]
    fn expands_home_in_identity_files() {
        let home = PathBuf::from("/Users/someone");
        let config = resolve_with_home(SAMPLE, "laptop", Some(&home)).unwrap();
        assert_eq!(
            config.identity_file.as_deref(),
            Some("/Users/someone/.ssh/id_ed25519")
        );
    }

    #[test]
    fn returns_nothing_for_an_unknown_alias_or_an_empty_block() {
        assert_eq!(resolve(SAMPLE, "unknown"), None);
        assert_eq!(resolve("Host x\n", "x"), None);
        assert_eq!(resolve("", "x"), None);
        assert_eq!(resolve(SAMPLE, "  "), None);
    }

    #[test]
    fn a_quoted_alias_resolves_to_its_first_word() {
        // `Host "quoted alias" spaced` lists two patterns; only an exact match
        // applies, so the quoted one is not reachable by typing it back.
        assert_eq!(resolve(SAMPLE, "quoted alias"), None);
        assert_eq!(
            resolve(SAMPLE, "spaced").unwrap().hostname.as_deref(),
            Some("space.example.com")
        );
    }

    #[test]
    fn parses_keyword_equals_value_and_ignores_unknown_keywords() {
        let text = "Host a\n  HostName=a.example\n  Compression yes\n  ServerAliveInterval 30\n";
        let config = resolve(text, "a").unwrap();
        assert_eq!(config.hostname.as_deref(), Some("a.example"));
    }

    #[test]
    fn later_blocks_do_not_override_earlier_ones() {
        let text = "Host prod\n  HostName first.example\nHost *\n  HostName second.example\n  User later\n";
        let config = resolve(text, "prod").unwrap();
        assert_eq!(config.hostname.as_deref(), Some("first.example"));
        assert_eq!(config.user, None);
    }

    #[test]
    fn reads_a_jump_target_with_a_user_and_port() {
        assert_eq!(
            jump_target("other-jump:2222"),
            (None, "other-jump".into(), Some(2222))
        );
        assert_eq!(
            jump_target("user@host"),
            (Some("user".into()), "host".into(), None)
        );
        assert_eq!(jump_target("host"), (None, "host".into(), None));
    }
}
