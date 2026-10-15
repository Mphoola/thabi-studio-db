-- Folders for saved queries. One level; '' means not in a folder.
ALTER TABLE saved_queries ADD COLUMN folder TEXT NOT NULL DEFAULT '';
CREATE INDEX saved_queries_folder ON saved_queries(folder COLLATE NOCASE, name COLLATE NOCASE);
