-- Phase 13: a saved query can have a trigger word that expands it in the
-- editor ("keyword binding"). '' means none. Unique when set.
ALTER TABLE saved_queries ADD COLUMN keyword TEXT NOT NULL DEFAULT '';
CREATE UNIQUE INDEX saved_queries_keyword ON saved_queries(keyword) WHERE keyword <> '';
