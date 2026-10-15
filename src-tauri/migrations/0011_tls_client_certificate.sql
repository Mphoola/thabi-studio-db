-- Mutual TLS: the client certificate and its private key are given as file
-- paths, never as contents, like SSH keys.
ALTER TABLE connection_profiles ADD COLUMN tls_client_cert TEXT NOT NULL DEFAULT '';
ALTER TABLE connection_profiles ADD COLUMN tls_client_key TEXT NOT NULL DEFAULT '';
