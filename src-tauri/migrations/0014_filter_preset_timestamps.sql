-- Give the preset timestamps a default.
--
-- The first version of the filter-presets migration created created_at and
-- updated_at as NOT NULL with no default, so nothing could be inserted into it:
-- an INSERT that does not name both columns fails. The migration file is
-- corrected, but a database that already applied it still has the broken
-- definition — a migration that has run cannot be changed by editing its file.
--
-- So the table is rebuilt here, which is the only way SQLite alters a column
-- default. Rows are carried across; there is nothing to lose if the old table
-- was unusable.
CREATE TABLE filter_presets_rebuilt (
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

INSERT INTO filter_presets_rebuilt (
    id, profile_id, database_name, schema_name, object_name, name,
    filters_json, created_at, updated_at
)
SELECT id, profile_id, database_name, schema_name, object_name, name,
       filters_json, created_at, updated_at
FROM filter_presets;

DROP TABLE filter_presets;

ALTER TABLE filter_presets_rebuilt RENAME TO filter_presets;

CREATE UNIQUE INDEX filter_presets_scope_name ON filter_presets (
    profile_id,
    database_name,
    schema_name,
    object_name,
    name COLLATE NOCASE
);