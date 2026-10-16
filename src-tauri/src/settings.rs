use crate::errors::AppError;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme_mode: String,
    pub sidebar_width: u16,
    pub sidebar_open: bool,
    pub selected_profile_id: Option<String>,
    /// 0 means no limit.
    pub query_timeout_seconds: u64,
    /// Rows fetched and kept per result.
    pub max_result_rows: u64,
    pub history_enabled: bool,
    /// 0 keeps history until the size cap.
    pub history_retention_days: u64,
    pub editor_font_size: u64,
    /// "" shows times as the server sent them; "local", "UTC" or an IANA
    /// zone converts values that carry an offset.
    pub display_time_zone: String,
    /// CSS font family for the SQL editor; "" uses the app's monospace font.
    pub editor_font_family: String,
    pub editor_tab_size: u64,
    pub editor_word_wrap: bool,
    /// "upper" or "lower": how completed keywords are written.
    pub editor_keyword_case: String,
    pub editor_vim: bool,
    /// Minutes idle before the app locks, when an app password is set; 0 never.
    pub auto_lock_minutes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyPreferences {
    pub theme_mode: Option<String>,
    pub sidebar_width: Option<u16>,
    pub sidebar_open: Option<bool>,
}

fn setting(connection: &Connection, key: &str) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| AppError::storage())
}

pub fn read_settings(connection: &Connection) -> Result<AppSettings, AppError> {
    let theme_mode = setting(connection, "theme_mode")?.ok_or_else(AppError::storage)?;
    if !matches!(theme_mode.as_str(), "system" | "light" | "dark") {
        return Err(AppError::storage());
    }

    let sidebar_width = setting(connection, "sidebar_width")?
        .ok_or_else(AppError::storage)?
        .parse::<u16>()
        .map_err(|_| AppError::storage())?;
    if !(200..=360).contains(&sidebar_width) {
        return Err(AppError::storage());
    }

    let sidebar_open = setting(connection, "sidebar_open")?
        .ok_or_else(AppError::storage)?
        .parse::<bool>()
        .map_err(|_| AppError::storage())?;

    Ok(AppSettings {
        theme_mode,
        sidebar_width,
        sidebar_open,
        selected_profile_id: setting(connection, "selected_profile_id")?
            .filter(|profile_id| !profile_id.is_empty()),
        query_timeout_seconds: setting(connection, "query_timeout_seconds")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(0),
        max_result_rows: setting(connection, "max_result_rows")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(10_000),
        history_enabled: setting(connection, "history_enabled")?.as_deref() != Some("false"),
        history_retention_days: setting(connection, "history_retention_days")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(30),
        editor_font_size: setting(connection, "editor_font_size")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(13),
        display_time_zone: setting(connection, "display_time_zone")?.unwrap_or_default(),
        editor_font_family: setting(connection, "editor_font_family")?.unwrap_or_default(),
        editor_tab_size: setting(connection, "editor_tab_size")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(2),
        editor_word_wrap: setting(connection, "editor_word_wrap")?.as_deref() == Some("true"),
        editor_keyword_case: setting(connection, "editor_keyword_case")?
            .filter(|case| matches!(case.as_str(), "upper" | "lower"))
            .unwrap_or_else(|| "upper".to_owned()),
        editor_vim: setting(connection, "editor_vim")?.as_deref() == Some("true"),
        auto_lock_minutes: setting(connection, "auto_lock_minutes")?
            .and_then(|value| value.parse().ok())
            .unwrap_or(15),
    })
}

pub fn import_preferences(
    connection: &mut Connection,
    preferences: LegacyPreferences,
) -> Result<AppSettings, AppError> {
    let transaction = connection.transaction().map_err(|_| AppError::storage())?;
    let imported = setting(&transaction, "legacy_preferences_imported")?
        .ok_or_else(AppError::storage)?
        .parse::<bool>()
        .map_err(|_| AppError::storage())?;

    if !imported {
        if let Some(theme_mode) = preferences.theme_mode {
            if !matches!(theme_mode.as_str(), "system" | "light" | "dark") {
                return Err(AppError::new(
                    "validation",
                    "Choose a supported appearance.",
                ));
            }
            transaction
                .execute(
                    "UPDATE app_settings SET value = ?1 WHERE key = 'theme_mode'",
                    [theme_mode],
                )
                .map_err(|_| AppError::storage())?;
        }
        if let Some(sidebar_width) = preferences.sidebar_width {
            if !(200..=360).contains(&sidebar_width) {
                return Err(AppError::new(
                    "validation",
                    "Choose a sidebar width from 200 to 360.",
                ));
            }
            transaction
                .execute(
                    "UPDATE app_settings SET value = ?1 WHERE key = 'sidebar_width'",
                    [sidebar_width.to_string()],
                )
                .map_err(|_| AppError::storage())?;
        }
        if let Some(sidebar_open) = preferences.sidebar_open {
            transaction
                .execute(
                    "UPDATE app_settings SET value = ?1 WHERE key = 'sidebar_open'",
                    [sidebar_open.to_string()],
                )
                .map_err(|_| AppError::storage())?;
        }
        transaction
            .execute(
                "UPDATE app_settings SET value = 'true' WHERE key = 'legacy_preferences_imported'",
                [],
            )
            .map_err(|_| AppError::storage())?;
    }

    transaction.commit().map_err(|_| AppError::storage())?;
    read_settings(connection)
}

/// Validates and stores one non-secret preference.
pub fn write_setting(connection: &Connection, key: &str, value: &Value) -> Result<(), AppError> {
    let value = match key {
        "theme_mode" => match value.as_str() {
            Some(mode @ ("system" | "light" | "dark")) => mode.to_owned(),
            _ => {
                return Err(AppError::new(
                    "validation",
                    "Choose a supported appearance.",
                ));
            }
        },
        "sidebar_width" => value
            .as_u64()
            .filter(|width| (200..=360).contains(width))
            .ok_or_else(|| AppError::new("validation", "Choose a sidebar width from 200 to 360."))?
            .to_string(),
        "sidebar_open" => value
            .as_bool()
            .map(|open| open.to_string())
            .ok_or_else(|| AppError::new("validation", "Choose whether the sidebar is open."))?,
        "history_enabled" => value
            .as_bool()
            .map(|on| on.to_string())
            .ok_or_else(|| AppError::new("validation", "Choose whether to keep history."))?,
        "history_retention_days" => value
            .as_u64()
            .filter(|days| *days <= 3650)
            .ok_or_else(|| AppError::new("validation", "Choose how long to keep history."))?
            .to_string(),
        "editor_font_size" => value
            .as_u64()
            .filter(|size| (10..=24).contains(size))
            .ok_or_else(|| AppError::new("validation", "Choose an editor font size."))?
            .to_string(),
        "max_result_rows" => value
            .as_u64()
            .filter(|rows| (1..=1_000_000).contains(rows))
            .ok_or_else(|| AppError::new("validation", "Choose how many rows to keep."))?
            .to_string(),
        "display_time_zone" => value
            .as_str()
            .filter(|zone| {
                zone.len() <= 64
                    && zone
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c))
            })
            .ok_or_else(|| AppError::new("validation", "Choose a time zone."))?
            .to_owned(),
        "editor_font_family" => value
            .as_str()
            .filter(|font| {
                font.chars().count() <= 80
                    && !font
                        .chars()
                        .any(|c| c.is_control() || ";{}<>\\".contains(c))
            })
            .ok_or_else(|| AppError::new("validation", "Enter a font family name."))?
            .to_owned(),
        "editor_tab_size" => value
            .as_u64()
            .filter(|size| (1..=8).contains(size))
            .ok_or_else(|| AppError::new("validation", "Choose a tab size from 1 to 8."))?
            .to_string(),
        "editor_word_wrap" | "editor_vim" => value
            .as_bool()
            .map(|on| on.to_string())
            .ok_or_else(|| AppError::new("validation", "Choose on or off."))?,
        "editor_keyword_case" => match value.as_str() {
            Some(case @ ("upper" | "lower")) => case.to_owned(),
            _ => return Err(AppError::new("validation", "Choose a keyword case.")),
        },
        "auto_lock_minutes" => value
            .as_u64()
            .filter(|minutes| *minutes <= 24 * 60)
            .ok_or_else(|| AppError::new("validation", "Choose when to lock."))?
            .to_string(),
        "query_timeout_seconds" => value
            .as_u64()
            .filter(|seconds| *seconds <= 24 * 60 * 60)
            .ok_or_else(|| AppError::new("validation", "Choose a query timeout."))?
            .to_string(),
        // Which connection groups are folded away on the connections page.
        "collapsed_groups" => {
            let groups = value
                .as_array()
                .filter(|groups| groups.len() <= 100)
                .ok_or_else(|| AppError::new("validation", "Choose which groups are collapsed."))?;
            let mut names = Vec::with_capacity(groups.len());
            for group in groups {
                let name = group.as_str().unwrap_or_default();
                if name.is_empty()
                    || name.chars().count() > 60
                    || name.chars().any(char::is_control)
                {
                    return Err(AppError::new(
                        "validation",
                        "Choose which groups are collapsed.",
                    ));
                }
                names.push(name.to_owned());
            }
            serde_json::to_string(&names).map_err(|_| AppError::storage())?
        }
        _ => {
            return Err(AppError::new(
                "validation",
                "This app preference can't be changed.",
            ));
        }
    };
    connection
        .execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )
        .map_err(|_| AppError::storage())?;
    Ok(())
}

pub fn select_profile(connection: &Connection, profile_id: Option<&str>) -> Result<(), AppError> {
    if let Some(profile_id) = profile_id {
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM connection_profiles WHERE id = ?1)",
                [profile_id],
                |row| row.get(0),
            )
            .map_err(|_| AppError::storage())?;
        if !exists {
            return Err(AppError::new(
                "not_found",
                "That saved connection couldn't be found.",
            ));
        }
    }
    connection
        .execute(
            "INSERT INTO app_settings (key, value) VALUES ('selected_profile_id', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [profile_id.unwrap_or("")],
        )
        .map_err(|_| AppError::storage())?;
    Ok(())
}

pub fn clear_selected_profile(
    transaction: &rusqlite::Transaction<'_>,
    profile_id: &str,
) -> Result<(), AppError> {
    let selected = transaction
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'selected_profile_id'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| AppError::storage())?;
    if selected.as_deref() == Some(profile_id) {
        transaction
            .execute(
                "DELETE FROM app_settings WHERE key = 'selected_profile_id'",
                [],
            )
            .map_err(|_| AppError::storage())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrate;
    use rusqlite::Connection;

    #[test]
    fn defaults_are_valid_and_selected_profile_is_optional() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();

        assert_eq!(
            read_settings(&connection).unwrap(),
            AppSettings {
                theme_mode: "system".into(),
                sidebar_width: 240,
                sidebar_open: true,
                selected_profile_id: None,
                query_timeout_seconds: 0,
                max_result_rows: 10_000,
                history_enabled: true,
                history_retention_days: 30,
                editor_font_size: 13,
                display_time_zone: String::new(),
                editor_font_family: String::new(),
                editor_tab_size: 2,
                editor_word_wrap: false,
                editor_keyword_case: "upper".into(),
                editor_vim: false,
                auto_lock_minutes: 15,
            }
        );
    }

    #[test]
    fn imports_legacy_preferences_only_once() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        let imported = import_preferences(
            &mut connection,
            LegacyPreferences {
                theme_mode: Some("dark".into()),
                sidebar_width: Some(280),
                sidebar_open: Some(false),
            },
        )
        .unwrap();
        assert_eq!(imported.theme_mode, "dark");
        assert_eq!(imported.sidebar_width, 280);
        assert!(!imported.sidebar_open);

        let unchanged = import_preferences(
            &mut connection,
            LegacyPreferences {
                theme_mode: Some("light".into()),
                sidebar_width: None,
                sidebar_open: None,
            },
        )
        .unwrap();
        assert_eq!(unchanged.theme_mode, "dark");
    }
}
