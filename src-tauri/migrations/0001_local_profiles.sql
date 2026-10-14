CREATE TABLE IF NOT EXISTS connection_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    environment TEXT NOT NULL CHECK (environment IN ('local', 'development', 'testing', 'staging', 'production')),
    driver TEXT NOT NULL CHECK (driver IN ('mysql', 'postgres', 'sqlserver')),
    host TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    database_name TEXT NOT NULL DEFAULT '',
    username TEXT NOT NULL,
    ssh_enabled INTEGER NOT NULL DEFAULT 0 CHECK (ssh_enabled IN (0, 1)),
    ssh_host TEXT NOT NULL DEFAULT '',
    ssh_port INTEGER NOT NULL DEFAULT 22 CHECK (ssh_port BETWEEN 1 AND 65535),
    ssh_username TEXT NOT NULL DEFAULT '',
    credential_key TEXT,
    has_database_password INTEGER NOT NULL DEFAULT 0 CHECK (has_database_password IN (0, 1)),
    has_ssh_password INTEGER NOT NULL DEFAULT 0 CHECK (has_ssh_password IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_connected_at TEXT,
    CHECK ((credential_key IS NOT NULL) OR (has_database_password = 0 AND has_ssh_password = 0))
);

CREATE INDEX IF NOT EXISTS connection_profiles_updated_at
    ON connection_profiles(updated_at DESC);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

INSERT OR IGNORE INTO app_settings (key, value) VALUES
    ('theme_mode', 'system'),
    ('sidebar_width', '240'),
    ('sidebar_open', 'true');
