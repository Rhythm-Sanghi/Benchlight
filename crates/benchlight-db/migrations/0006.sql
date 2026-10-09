CREATE TABLE working_snapshots (
    id INTEGER PRIMARY KEY,
    project TEXT NOT NULL COLLATE NOCASE,
    created_at INTEGER NOT NULL,
    detail TEXT NOT NULL
);
CREATE INDEX working_snapshots_project ON working_snapshots(project, id DESC);
PRAGMA user_version = 6;
