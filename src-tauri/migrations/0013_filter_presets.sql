-- Named filter sets for one table or view.
--
-- A lookup the user works at often should not have to be built again, and it
-- belongs to the table rather than to a tab: it should still be there after the
-- tab is closed and the app restarted. Scoped to a connection and object, since
-- "recent orders" means nothing on another server or another table.
CREATE TABLE filter_presets (
    id TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES connection_profiles(id) ON DELETE CASCADE,
    database_name TEXT NOT NULL,
    schema_name TEXT NOT NULL DEFAULT '',
    object_name TEXT NOT NULL,
    name TEXT NOT NULL,
    -- The filter set as JSON, so a preset is whatever the filter bar held.
    filters_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- One preset per name within a table, and names are compared the way a person
-- reads them: "Recent orders" and "recent orders" are the same preset.
CREATE UNIQUE INDEX filter_presets_scope_name ON filter_presets (
    profile_id,
    database_name,
    schema_name,
    object_name,
    name COLLATE NOCASE
);