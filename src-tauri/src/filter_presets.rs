//! Saved filter presets: the filter sets a table is usually looked at with.
//!
//! A preset belongs to a table on a connection, not to a tab, so it is still
//! there after the tab is closed and the app restarted. The filters are kept as
//! the JSON the webview already uses, so applying one is handing the filter bar
//! back what it held before rather than translating it.

use crate::errors::AppError;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

fn storage(_: rusqlite::Error) -> AppError {
    AppError::storage()
}

/// One saved filter set.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterPreset {
    pub id: String,
    pub profile_id: String,
    pub database_name: String,
    pub schema_name: String,
    pub object_name: String,
    pub name: String,
    /// The filter set as the webview sends it: `[{ column, op, value }]`.
    pub filters: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterPresetInput {
    /// Set to update a preset instead of adding one.
    #[serde(default)]
    pub id: Option<String>,
    pub profile_id: String,
    pub database_name: String,
    #[serde(default)]
    pub schema_name: String,
    pub object_name: String,
    pub name: String,
    pub filters: serde_json::Value,
}

/// A preset with no filters would show as a name that does nothing.
const MAX_FILTERS: usize = 50;

fn field_error(field: &str, message: &str) -> AppError {
    let mut fields = std::collections::BTreeMap::new();
    fields.insert(field.to_owned(), message.to_owned());
    AppError::validation(fields)
}

/// Whether the stored JSON is a non-empty list of filter objects, which is all
/// that is read back. An empty preset would be a name in the list that shows
/// every row when chosen, which is not what anyone means by a saved lookup.
fn valid_filters(filters: &serde_json::Value) -> bool {
    filters.as_array().is_some_and(|list| {
        !list.is_empty() && list.len() <= MAX_FILTERS && list.iter().all(|f| f.is_object())
    })
}

fn preset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FilterPreset> {
    let json: String = row.get(6)?;
    Ok(FilterPreset {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        database_name: row.get(2)?,
        schema_name: row.get(3)?,
        object_name: row.get(4)?,
        name: row.get(5)?,
        // A row that will not parse is not worth failing the whole list over.
        filters: serde_json::from_str(&json).unwrap_or(serde_json::Value::Array(vec![])),
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

const PRESET_COLUMNS: &str = "id, profile_id, database_name, schema_name, object_name, name, \
     filters_json, created_at, updated_at";

/// Every preset saved for one table, in the order they are offered.
pub fn list(
    connection: &Connection,
    profile_id: &str,
    database_name: &str,
    schema_name: &str,
    object_name: &str,
) -> Result<Vec<FilterPreset>, AppError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT {PRESET_COLUMNS} FROM filter_presets
             WHERE profile_id = ?1 AND database_name = ?2 AND schema_name = ?3
               AND object_name = ?4
             ORDER BY name COLLATE NOCASE, id"
        ))
        .map_err(storage)?;
    let rows = statement
        .query_map(
            params![profile_id, database_name, schema_name, object_name],
            preset_from_row,
        )
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    Ok(rows)
}

/// Saves a preset, or updates the one named by `id`.
///
/// A name already used on the same table is that preset, updated: saving
/// "Recent orders" twice is the same filter set getting fresher, not two.
pub fn save(connection: &Connection, input: FilterPresetInput) -> Result<FilterPreset, AppError> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return Err(field_error("name", "Enter a name of up to 120 characters."));
    }
    if input.object_name.trim().is_empty() {
        return Err(field_error("objectName", "A preset needs a table."));
    }
    if !valid_filters(&input.filters) {
        return Err(field_error(
            "filters",
            "Save at least one filter, and no more than 50.",
        ));
    }
    let filters = serde_json::to_string(&input.filters)
        .map_err(|_| AppError::new("invalid", "Those filters could not be saved."))?;

    let id = match input.id {
        Some(id) => {
            let changed = connection
                .execute(
                    "UPDATE filter_presets SET name = ?2, filters_json = ?3,
                        updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?1",
                    params![id, name, filters],
                )
                .map_err(storage)?;
            if changed == 0 {
                return Err(AppError::new(
                    "not_found",
                    "That filter preset no longer exists.",
                ));
            }
            id
        }
        None => {
            // Reusing the name on the same table updates that preset, so the
            // unique index cannot be hit and a person cannot end up with two
            // presets they cannot tell apart.
            let existing: Option<String> = connection
                .query_row(
                    "SELECT id FROM filter_presets
                     WHERE profile_id = ?1 AND database_name = ?2 AND schema_name = ?3
                       AND object_name = ?4 AND name = ?5 COLLATE NOCASE",
                    params![
                        input.profile_id,
                        input.database_name,
                        input.schema_name,
                        input.object_name,
                        name
                    ],
                    |row| row.get(0),
                )
                .ok();
            match existing {
                Some(id) => {
                    // The name is written too: saving over an existing preset
                    // stores what was just typed, rather than silently keeping
                    // the spelling from whenever it was first saved.
                    connection
                        .execute(
                            "UPDATE filter_presets SET name = ?2, filters_json = ?3,
                                updated_at = CURRENT_TIMESTAMP
                             WHERE id = ?1",
                            params![id, name, filters],
                        )
                        .map_err(storage)?;
                    id
                }
                None => {
                    let id: String = connection
                        .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
                        .map_err(storage)?;
                    connection
                        .execute(
                            "INSERT INTO filter_presets (id, profile_id, database_name,
                                schema_name, object_name, name, filters_json)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                            params![
                                id,
                                input.profile_id,
                                input.database_name,
                                input.schema_name,
                                input.object_name,
                                name,
                                filters
                            ],
                        )
                        .map_err(storage)?;
                    id
                }
            }
        }
    };

    connection
        .query_row(
            &format!("SELECT {PRESET_COLUMNS} FROM filter_presets WHERE id = ?1"),
            [id],
            preset_from_row,
        )
        .map_err(storage)
}

/// Deletes a preset.
pub fn delete(connection: &Connection, id: &str) -> Result<(), AppError> {
    let changed = connection
        .execute("DELETE FROM filter_presets WHERE id = ?1", [id])
        .map_err(storage)?;
    if changed == 0 {
        return Err(AppError::new(
            "not_found",
            "That filter preset no longer exists.",
        ));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrate;
    use serde_json::json;

    fn db() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO connection_profiles (id, name, environment, driver, host, port, username)
                 VALUES ('shop', 'Shop', 'production', 'postgres', 'localhost', 5432, 'user')",
                [],
            )
            .unwrap();
        connection
    }

    fn input(name: &str) -> FilterPresetInput {
        FilterPresetInput {
            id: None,
            profile_id: "shop".into(),
            database_name: "public".into(),
            schema_name: "public".into(),
            object_name: "orders".into(),
            name: name.into(),
            filters: json!([{ "column": "status", "op": "eq", "value": "open" }]),
        }
    }

    #[test]
    fn saves_and_lists_a_table_presets_in_name_order() {
        let connection = db();
        save(&connection, input("Recent orders")).unwrap();
        save(&connection, input("Awaiting payment")).unwrap();
        let presets = list(&connection, "shop", "public", "public", "orders").unwrap();
        let names: Vec<_> = presets.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["Awaiting payment", "Recent orders"]);
        assert_eq!(presets[1].filters[0]["column"], "status");
    }

    #[test]
    fn a_preset_is_only_offered_on_its_own_table() {
        let connection = db();
        save(&connection, input("Recent")).unwrap();
        assert!(
            list(&connection, "shop", "public", "public", "customers")
                .unwrap()
                .is_empty()
        );
        // Same table name, another connection or another schema, is not it.
        assert!(
            list(&connection, "other", "public", "public", "orders")
                .unwrap()
                .is_empty()
        );
        assert!(
            list(&connection, "shop", "public", "private", "orders")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn saving_a_name_twice_updates_that_preset() {
        let connection = db();
        let first = save(&connection, input("Recent orders")).unwrap();
        let mut second_input = input("recent orders");
        second_input.filters = json!([{ "column": "total", "op": "gt", "value": "100" }]);
        let second = save(&connection, second_input).unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.name, "recent orders");
        assert_eq!(second.filters[0]["column"], "total");
        assert_eq!(
            list(&connection, "shop", "public", "public", "orders")
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn a_named_preset_can_be_replaced() {
        let connection = db();
        let saved = save(&connection, input("Recent")).unwrap();
        let mut update = input("Something else");
        update.id = Some(saved.id.clone());
        let updated = save(&connection, update).unwrap();
        assert_eq!(updated.id, saved.id);
        assert_eq!(updated.name, "Something else");
    }

    #[test]
    fn refuses_a_preset_with_no_name_or_no_table() {
        let connection = db();
        assert!(save(&connection, input("   ")).is_err());
        let mut nameless = input("x");
        nameless.object_name = " ".into();
        assert!(save(&connection, nameless).is_err());
        let mut long = input("x");
        long.name = "n".repeat(121);
        assert!(save(&connection, long).is_err());
    }

    #[test]
    fn refuses_filters_that_are_not_a_list_of_filters() {
        let connection = db();
        let mut none = input("Empty");
        none.filters = json!([]);
        assert!(save(&connection, none).is_err());
        let mut not_a_list = input("Odd");
        not_a_list.filters = json!({ "column": "status" });
        assert!(save(&connection, not_a_list).is_err());
        let mut not_filters = input("Odd");
        not_filters.filters = json!(["status = open"]);
        assert!(save(&connection, not_filters).is_err());
        let mut too_many = input("Too many");
        too_many.filters = json!(vec![json!({ "column": "a" }); 51]);
        assert!(save(&connection, too_many).is_err());
    }

    #[test]
    fn deleting_reports_a_preset_that_is_already_gone() {
        let connection = db();
        let saved = save(&connection, input("Recent")).unwrap();
        delete(&connection, &saved.id).unwrap();
        assert!(delete(&connection, &saved.id).is_err());
        assert!(
            save(
                &connection,
                FilterPresetInput {
                    id: Some("nope".into()),
                    ..input("x")
                }
            )
            .is_err()
        );
        assert!(
            list(&connection, "shop", "public", "public", "orders")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn timestamps_default_so_a_preset_can_be_saved_without_naming_them() {
        // 0013 created these without a default, which made every INSERT fail on
        // a database that had already applied it; 0014 rebuilds the table. The
        // default is what lets the save below work at all, so it is asserted on
        // the schema rather than inferred from the insert succeeding.
        let connection = db();
        for column in ["created_at", "updated_at"] {
            let default: Option<String> = connection
                .query_row(
                    "SELECT dflt_value FROM pragma_table_info('filter_presets')
                     WHERE name = ?1",
                    [column],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(
                default.as_deref(),
                Some("CURRENT_TIMESTAMP"),
                "{column} has no default"
            );
        }
        save(&connection, input("Recent")).unwrap();
    }

    #[test]
    fn a_migration_over_an_existing_table_keeps_what_it_had() {
        // A database that ran the broken first version and then 0014: the rows
        // it managed to keep are still there.
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        connection
            .execute(
                "INSERT INTO connection_profiles (id, name, environment, driver, host, port, username)
                 VALUES ('shop', 'Shop', 'production', 'postgres', 'localhost', 5432, 'user')",
                [],
            )
            .unwrap();
        save(&connection, input("Kept")).unwrap();
        let presets = list(&connection, "shop", "public", "public", "orders").unwrap();
        assert_eq!(presets[0].name, "Kept");
    }

    #[test]
    fn a_preset_goes_when_its_connection_does() {
        let connection = db();
        save(&connection, input("Recent")).unwrap();
        connection
            .execute("DELETE FROM connection_profiles WHERE id = 'shop'", [])
            .unwrap();
        assert!(
            list(&connection, "shop", "public", "public", "orders")
                .unwrap()
                .is_empty()
        );
    }
}
