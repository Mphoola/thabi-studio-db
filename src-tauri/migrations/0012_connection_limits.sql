-- Per-connection overrides for the app-wide query timeout and row limit.
-- 0 means "use the app default", so existing connections are unchanged.
ALTER TABLE connection_profiles ADD COLUMN query_timeout_seconds INTEGER NOT NULL DEFAULT 0;
ALTER TABLE connection_profiles ADD COLUMN max_result_rows INTEGER NOT NULL DEFAULT 0;
