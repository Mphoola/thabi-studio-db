-- Phase 9: query history, saved queries and open-tab recovery.

CREATE TABLE query_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id TEXT,
    connection_name TEXT NOT NULL DEFAULT '',
    database_name TEXT NOT NULL DEFAULT '',
    query_text TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    duration_ms INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL CHECK (status IN ('completed', 'failed', 'cancelled', 'timed_out')),
    row_count INTEGER,
    error_summary TEXT
);
CREATE INDEX query_history_started_at ON query_history(started_at DESC);
CREATE INDEX query_history_profile ON query_history(profile_id, started_at DESC);

CREATE TABLE saved_queries (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    query_text TEXT NOT NULL,
    profile_id TEXT,
    notes TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX saved_queries_name ON saved_queries(name COLLATE NOCASE);

-- Open query tabs, so unsaved SQL survives quitting or a crash.
CREATE TABLE workspace_tabs (
    position INTEGER PRIMARY KEY NOT NULL,
    tab_id TEXT NOT NULL,
    title TEXT NOT NULL,
    query_text TEXT NOT NULL,
    profile_id TEXT,
    database_name TEXT,
    saved_query_id TEXT
);

INSERT OR IGNORE INTO app_settings (key, value) VALUES
    ('history_enabled', 'true'),
    ('history_retention_days', '30'),
    ('editor_font_size', '13');
