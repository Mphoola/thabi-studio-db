-- Custom CA certificate for "verify-full" connections. Empty means the
-- system's trusted certificates.
ALTER TABLE connection_profiles ADD COLUMN tls_ca_path TEXT NOT NULL DEFAULT '';
