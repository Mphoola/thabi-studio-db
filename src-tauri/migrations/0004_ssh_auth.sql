-- SSH key and agent authentication, and trusted SSH host keys.
-- SQLite can't change a CHECK constraint in place, so connection_profiles
-- is rebuilt with ssh_auth_mode allowing 'password', 'key' and 'agent'.

CREATE TABLE connection_profiles_new (
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
    auth_mode TEXT NOT NULL DEFAULT 'password' CHECK (auth_mode = 'password'),
    tls_mode TEXT NOT NULL DEFAULT 'verify-full' CHECK (tls_mode IN ('verify-full', 'require', 'disable')),
    ssh_auth_mode TEXT NOT NULL DEFAULT 'password' CHECK (ssh_auth_mode IN ('password', 'key', 'agent')),
    tls_ca_path TEXT NOT NULL DEFAULT '',
    ssh_key_path TEXT NOT NULL DEFAULT '',
    has_ssh_key_passphrase INTEGER NOT NULL DEFAULT 0 CHECK (has_ssh_key_passphrase IN (0, 1)),
    CHECK ((credential_key IS NOT NULL) OR (has_database_password = 0 AND has_ssh_password = 0 AND has_ssh_key_passphrase = 0))
);

INSERT INTO connection_profiles_new (
    id, name, environment, driver, host, port, database_name, username,
    ssh_enabled, ssh_host, ssh_port, ssh_username, credential_key,
    has_database_password, has_ssh_password, created_at, updated_at,
    last_connected_at, auth_mode, tls_mode, ssh_auth_mode, tls_ca_path
)
SELECT
    id, name, environment, driver, host, port, database_name, username,
    ssh_enabled, ssh_host, ssh_port, ssh_username, credential_key,
    has_database_password, has_ssh_password, created_at, updated_at,
    last_connected_at, auth_mode, tls_mode, ssh_auth_mode, tls_ca_path
FROM connection_profiles;

DROP TABLE connection_profiles;
ALTER TABLE connection_profiles_new RENAME TO connection_profiles;

CREATE INDEX IF NOT EXISTS connection_profiles_updated_at
    ON connection_profiles(updated_at DESC);

CREATE TABLE ssh_known_hosts (
    host TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    algorithm TEXT NOT NULL,
    public_key TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (host, port, algorithm)
);
