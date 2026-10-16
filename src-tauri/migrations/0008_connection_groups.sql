-- User-named groups for connections, a manual order inside each group, and an
-- optional colour token. Empty group_name and colour mean "not set": the
-- connection keeps its environment group and tag colour.
ALTER TABLE connection_profiles ADD COLUMN group_name TEXT NOT NULL DEFAULT '';
ALTER TABLE connection_profiles ADD COLUMN colour TEXT NOT NULL DEFAULT '';
ALTER TABLE connection_profiles ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
CREATE INDEX connection_profiles_group
  ON connection_profiles(group_name COLLATE NOCASE, sort_order, name COLLATE NOCASE);
