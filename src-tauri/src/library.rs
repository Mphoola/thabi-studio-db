//! Query history, saved queries and open-tab recovery, stored in the app's
//! SQLite database. History holds SQL text, which can contain sensitive
//! literals: it can be turned off, is pruned by age and size, and can be
//! cleared (see docs/PRIVACY.md).

use crate::errors::AppError;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

/// History never grows past this many entries, whatever the retention.
const HISTORY_CAP: i64 = 10_000;
/// Longest SQL kept in history (characters); longer text is cut.
const HISTORY_SQL_CHARS: usize = 100_000;

fn storage(_: rusqlite::Error) -> AppError {
    AppError::storage()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: i64,
    pub profile_id: Option<String>,
    pub connection_name: String,
    pub database_name: String,
    pub query_text: String,
    pub started_at: String,
    pub duration_ms: i64,
    pub status: String,
    pub row_count: Option<i64>,
    pub error_summary: Option<String>,
}

pub struct NewHistoryEntry<'a> {
    pub profile_id: &'a str,
    pub connection_name: &'a str,
    pub database_name: &'a str,
    pub query_text: &'a str,
    pub duration_ms: i64,
    pub status: &'static str,
    pub row_count: Option<i64>,
    pub error_summary: Option<&'a str>,
}

fn setting(connection: &Connection, key: &str) -> Result<Option<String>, AppError> {
    connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage)
}

/// Records a run, unless history is turned off, then prunes old entries.
pub fn record(connection: &Connection, entry: &NewHistoryEntry<'_>) -> Result<(), AppError> {
    if setting(connection, "history_enabled")?.as_deref() == Some("false") {
        return Ok(());
    }
    let text: String = entry.query_text.chars().take(HISTORY_SQL_CHARS).collect();
    connection
        .execute(
            "INSERT INTO query_history (profile_id, connection_name, database_name, query_text,
                duration_ms, status, row_count, error_summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry.profile_id,
                entry.connection_name,
                entry.database_name,
                text,
                entry.duration_ms,
                entry.status,
                entry.row_count,
                entry.error_summary,
            ],
        )
        .map_err(storage)?;
    prune(connection)
}

/// Removes entries older than the retention setting and beyond the cap.
pub fn prune(connection: &Connection) -> Result<(), AppError> {
    let days: i64 = setting(connection, "history_retention_days")?
        .and_then(|d| d.parse().ok())
        .unwrap_or(30);
    if days > 0 {
        connection
            .execute(
                "DELETE FROM query_history WHERE started_at < datetime('now', ?1)",
                [format!("-{days} days")],
            )
            .map_err(storage)?;
    }
    connection
        .execute(
            "DELETE FROM query_history WHERE id NOT IN
               (SELECT id FROM query_history ORDER BY id DESC LIMIT ?1)",
            [HISTORY_CAP],
        )
        .map_err(storage)?;
    Ok(())
}

/// Newest first. `search` matches SQL, connection or database names.
pub fn list_history(
    connection: &Connection,
    search: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<HistoryEntry>, AppError> {
    let pattern = format!(
        "%{}%",
        search
            .trim()
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    );
    let mut statement = connection
        .prepare(
            "SELECT id, profile_id, connection_name, database_name, query_text, started_at,
                    duration_ms, status, row_count, error_summary
             FROM query_history
             WHERE query_text LIKE ?1 ESCAPE '\\' OR connection_name LIKE ?1 ESCAPE '\\'
                OR database_name LIKE ?1 ESCAPE '\\'
             ORDER BY id DESC LIMIT ?2 OFFSET ?3",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(
            params![pattern, limit.clamp(1, 500), offset.max(0)],
            |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    profile_id: row.get(1)?,
                    connection_name: row.get(2)?,
                    database_name: row.get(3)?,
                    query_text: row.get(4)?,
                    started_at: row.get(5)?,
                    duration_ms: row.get(6)?,
                    status: row.get(7)?,
                    row_count: row.get(8)?,
                    error_summary: row.get(9)?,
                })
            },
        )
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    Ok(rows)
}

pub fn delete_history_entry(connection: &Connection, id: i64) -> Result<(), AppError> {
    connection
        .execute("DELETE FROM query_history WHERE id = ?1", [id])
        .map_err(storage)?;
    Ok(())
}

pub fn clear_history(connection: &Connection) -> Result<usize, AppError> {
    connection
        .execute("DELETE FROM query_history", [])
        .map_err(storage)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQuery {
    pub id: String,
    pub name: String,
    pub query_text: String,
    pub profile_id: Option<String>,
    pub notes: String,
    pub folder: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQueryInput {
    /// Present to update an existing saved query.
    pub id: Option<String>,
    pub name: String,
    pub query_text: String,
    pub profile_id: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub folder: String,
}

fn saved_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedQuery> {
    Ok(SavedQuery {
        id: row.get(0)?,
        name: row.get(1)?,
        query_text: row.get(2)?,
        profile_id: row.get(3)?,
        notes: row.get(4)?,
        folder: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

const SAVED_COLUMNS: &str =
    "id, name, query_text, profile_id, notes, folder, created_at, updated_at";

pub fn list_saved(connection: &Connection) -> Result<Vec<SavedQuery>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {SAVED_COLUMNS} FROM saved_queries
             ORDER BY folder COLLATE NOCASE, name COLLATE NOCASE, id"
        ))
        .map_err(storage)?;
    let rows = statement
        .query_map([], saved_from_row)
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    Ok(rows)
}

fn field_error(field: &str, message: &str) -> AppError {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert(field.to_owned(), message.to_owned());
    AppError::validation(fields)
}

/// Trims a folder name; '' means no folder.
fn clean_folder(folder: &str) -> Result<&str, AppError> {
    let folder = folder.trim();
    if folder.chars().count() > 80 {
        return Err(field_error(
            "folder",
            "Use a folder name of up to 80 characters.",
        ));
    }
    Ok(folder)
}

pub fn save_query(connection: &Connection, input: SavedQueryInput) -> Result<SavedQuery, AppError> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(field_error("name", "Enter a name of up to 120 characters."));
    }
    let folder = clean_folder(&input.folder)?;
    let id = match input.id {
        Some(id) => {
            let changed = connection
                .execute(
                    "UPDATE saved_queries SET name = ?2, query_text = ?3, profile_id = ?4,
                        notes = ?5, folder = ?6, updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![
                        id,
                        name,
                        input.query_text,
                        input.profile_id,
                        input.notes,
                        folder
                    ],
                )
                .map_err(storage)?;
            if changed == 0 {
                return Err(AppError::new(
                    "not_found",
                    "That saved query no longer exists.",
                ));
            }
            id
        }
        None => {
            let id: String = connection
                .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
                .map_err(storage)?;
            connection
                .execute(
                    "INSERT INTO saved_queries (id, name, query_text, profile_id, notes, folder)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        id,
                        name,
                        input.query_text,
                        input.profile_id,
                        input.notes,
                        folder
                    ],
                )
                .map_err(storage)?;
            id
        }
    };
    connection
        .query_row(
            &format!("SELECT {SAVED_COLUMNS} FROM saved_queries WHERE id = ?1"),
            [id],
            saved_from_row,
        )
        .map_err(storage)
}

/// Renames a folder, merging into `to` if it already exists. An empty `to`
/// takes the queries out of the folder. Returns how many queries moved.
pub fn rename_folder(connection: &Connection, from: &str, to: &str) -> Result<usize, AppError> {
    let to = clean_folder(to)?;
    connection
        .execute(
            "UPDATE saved_queries SET folder = ?2, updated_at = CURRENT_TIMESTAMP
             WHERE folder = ?1",
            params![from, to],
        )
        .map_err(storage)
}

pub fn delete_saved(connection: &Connection, id: &str) -> Result<(), AppError> {
    connection
        .execute("DELETE FROM saved_queries WHERE id = ?1", [id])
        .map_err(storage)?;
    Ok(())
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTab {
    pub tab_id: String,
    pub title: String,
    pub query_text: String,
    pub profile_id: Option<String>,
    pub database_name: Option<String>,
    pub saved_query_id: Option<String>,
}

/// Replaces the stored open tabs in one transaction.
pub fn save_tabs(connection: &mut Connection, tabs: &[WorkspaceTab]) -> Result<(), AppError> {
    let transaction = connection.transaction().map_err(storage)?;
    transaction
        .execute("DELETE FROM workspace_tabs", [])
        .map_err(storage)?;
    for (position, tab) in tabs.iter().take(100).enumerate() {
        transaction
            .execute(
                "INSERT INTO workspace_tabs (position, tab_id, title, query_text, profile_id,
                    database_name, saved_query_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    position as i64,
                    tab.tab_id,
                    tab.title,
                    tab.query_text,
                    tab.profile_id,
                    tab.database_name,
                    tab.saved_query_id,
                ],
            )
            .map_err(storage)?;
    }
    transaction.commit().map_err(storage)
}

pub fn load_tabs(connection: &Connection) -> Result<Vec<WorkspaceTab>, AppError> {
    let mut statement = connection
        .prepare(
            "SELECT tab_id, title, query_text, profile_id, database_name, saved_query_id
             FROM workspace_tabs ORDER BY position",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map([], |row| {
            Ok(WorkspaceTab {
                tab_id: row.get(0)?,
                title: row.get(1)?,
                query_text: row.get(2)?,
                profile_id: row.get(3)?,
                database_name: row.get(4)?,
                saved_query_id: row.get(5)?,
            })
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrate;

    fn db() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
    }

    fn entry(sql: &str) -> NewHistoryEntry<'_> {
        NewHistoryEntry {
            profile_id: "p",
            connection_name: "Local",
            database_name: "shop",
            query_text: sql,
            duration_ms: 12,
            status: "completed",
            row_count: Some(3),
            error_summary: None,
        }
    }

    #[test]
    fn records_lists_searches_and_clears_history() {
        let connection = db();
        record(&connection, &entry("SELECT 1")).unwrap();
        record(&connection, &entry("SELECT * FROM orders")).unwrap();
        let all = list_history(&connection, "", 50, 0).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].query_text, "SELECT * FROM orders", "newest first");
        assert_eq!(list_history(&connection, "orders", 50, 0).unwrap().len(), 1);
        // LIKE wildcards in the search are literal.
        assert_eq!(list_history(&connection, "%", 50, 0).unwrap().len(), 0);
        delete_history_entry(&connection, all[0].id).unwrap();
        assert_eq!(list_history(&connection, "", 50, 0).unwrap().len(), 1);
        assert_eq!(clear_history(&connection).unwrap(), 1);
    }

    #[test]
    fn history_can_be_turned_off_and_old_entries_are_pruned() {
        let connection = db();
        connection
            .execute(
                "UPDATE app_settings SET value = 'false' WHERE key = 'history_enabled'",
                [],
            )
            .unwrap();
        record(&connection, &entry("SELECT 1")).unwrap();
        assert!(list_history(&connection, "", 50, 0).unwrap().is_empty());

        connection
            .execute(
                "UPDATE app_settings SET value = 'true' WHERE key = 'history_enabled'",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO query_history (query_text, status, started_at)
                 VALUES ('old', 'completed', datetime('now', '-40 days'))",
                [],
            )
            .unwrap();
        record(&connection, &entry("SELECT 2")).unwrap();
        let left = list_history(&connection, "", 50, 0).unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].query_text, "SELECT 2");
    }

    #[test]
    fn saves_updates_and_deletes_queries() {
        let connection = db();
        let saved = save_query(
            &connection,
            SavedQueryInput {
                id: None,
                name: " Pending orders ".into(),
                query_text: "SELECT 1".into(),
                profile_id: None,
                notes: String::new(),
                folder: " Reports ".into(),
            },
        )
        .unwrap();
        assert_eq!(saved.name, "Pending orders");
        assert_eq!(saved.folder, "Reports");
        let updated = save_query(
            &connection,
            SavedQueryInput {
                id: Some(saved.id.clone()),
                name: "Pending".into(),
                query_text: "SELECT 2".into(),
                profile_id: Some("p".into()),
                notes: "daily".into(),
                folder: "Reports".into(),
            },
        )
        .unwrap();
        assert_eq!(updated.query_text, "SELECT 2");
        assert_eq!(list_saved(&connection).unwrap().len(), 1);
        delete_saved(&connection, &saved.id).unwrap();
        assert!(list_saved(&connection).unwrap().is_empty());
        assert_eq!(
            save_query(
                &connection,
                SavedQueryInput {
                    id: None,
                    name: " ".into(),
                    query_text: String::new(),
                    profile_id: None,
                    notes: String::new(),
                    folder: String::new(),
                },
            )
            .unwrap_err()
            .kind,
            "validation"
        );
    }

    #[test]
    fn folders_rename_merge_and_clear() {
        let mut connection = db();
        let add = |connection: &Connection, name: &str, folder: &str| {
            save_query(
                connection,
                SavedQueryInput {
                    id: None,
                    name: name.into(),
                    query_text: "SELECT 1".into(),
                    profile_id: None,
                    notes: String::new(),
                    folder: folder.into(),
                },
            )
            .unwrap()
        };
        let connection = &mut connection;
        add(connection, "b", "Reports");
        add(connection, "a", "Ops");
        add(connection, "c", "");
        let order = |connection: &Connection| {
            list_saved(connection)
                .unwrap()
                .into_iter()
                .map(|q| format!("{}/{}", q.folder, q.name))
                .collect::<Vec<_>>()
        };
        assert_eq!(order(connection), ["/c", "Ops/a", "Reports/b"]);
        assert_eq!(rename_folder(connection, "Ops", "Reports").unwrap(), 1);
        assert_eq!(order(connection), ["/c", "Reports/a", "Reports/b"]);
        assert_eq!(rename_folder(connection, "Reports", " ").unwrap(), 2);
        assert!(
            list_saved(connection)
                .unwrap()
                .iter()
                .all(|q| q.folder.is_empty())
        );
        assert_eq!(
            rename_folder(connection, "", &"x".repeat(81))
                .unwrap_err()
                .kind,
            "validation"
        );
    }

    #[test]
    fn open_tabs_round_trip_in_order() {
        let mut connection = db();
        let tabs = vec![
            WorkspaceTab {
                tab_id: "a".into(),
                title: "Query 1".into(),
                query_text: "SELECT 1".into(),
                profile_id: Some("p".into()),
                database_name: Some("shop".into()),
                saved_query_id: None,
            },
            WorkspaceTab {
                tab_id: "b".into(),
                title: "Report".into(),
                query_text: "SELECT 2".into(),
                profile_id: None,
                database_name: None,
                saved_query_id: Some("s".into()),
            },
        ];
        save_tabs(&mut connection, &tabs).unwrap();
        assert_eq!(load_tabs(&connection).unwrap(), tabs);
        save_tabs(&mut connection, &[]).unwrap();
        assert!(load_tabs(&connection).unwrap().is_empty());
    }
}
