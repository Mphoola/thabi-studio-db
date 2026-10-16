-- Secrets the user prefers to type at connect time rather than keep in the OS
-- credential store: nothing is saved for these, and the app asks each time.
ALTER TABLE connection_profiles ADD COLUMN ask_database_password INTEGER NOT NULL DEFAULT 0;
ALTER TABLE connection_profiles ADD COLUMN ask_ssh_password INTEGER NOT NULL DEFAULT 0;
ALTER TABLE connection_profiles ADD COLUMN ask_ssh_key_passphrase INTEGER NOT NULL DEFAULT 0;
ALTER TABLE connection_profiles ADD COLUMN ssh_keepalive_seconds INTEGER NOT NULL DEFAULT 0;
