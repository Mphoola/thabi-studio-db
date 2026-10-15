-- Safe mode per connection: "off", "confirm-writes", "confirm-everything"
-- or "read-only". Existing Production connections keep today's behaviour by
-- starting on "confirm-writes"; everything else starts on "off".
ALTER TABLE connection_profiles ADD COLUMN safe_mode TEXT NOT NULL DEFAULT 'off';
UPDATE connection_profiles SET safe_mode = 'confirm-writes' WHERE environment = 'production';
