ALTER TABLE connection_profiles
    ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'password'
    CHECK (auth_mode = 'password');

ALTER TABLE connection_profiles
    ADD COLUMN tls_mode TEXT NOT NULL DEFAULT 'verify-full'
    CHECK (tls_mode IN ('verify-full', 'require', 'disable'));

ALTER TABLE connection_profiles
    ADD COLUMN ssh_auth_mode TEXT NOT NULL DEFAULT 'password'
    CHECK (ssh_auth_mode = 'password');

INSERT OR IGNORE INTO app_settings (key, value)
    VALUES ('legacy_preferences_imported', 'false');
