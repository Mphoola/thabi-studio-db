-- Statements to run on every new session for this connection, one per line.
ALTER TABLE connection_profiles ADD COLUMN startup_commands TEXT NOT NULL DEFAULT '';
