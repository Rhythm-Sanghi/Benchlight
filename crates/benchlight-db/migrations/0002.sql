CREATE TABLE scan_runs (
    id INTEGER PRIMARY KEY,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    state TEXT NOT NULL,
    directories INTEGER NOT NULL DEFAULT 0,
    projects INTEGER NOT NULL DEFAULT 0,
    errors INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE projects (
    scan_id INTEGER NOT NULL REFERENCES scan_runs(id) ON DELETE CASCADE,
    path TEXT NOT NULL COLLATE NOCASE,
    name TEXT NOT NULL,
    evidence TEXT NOT NULL,
    languages TEXT NOT NULL,
    last_activity INTEGER,
    PRIMARY KEY (scan_id, path)
);
CREATE INDEX projects_name ON projects(scan_id, name COLLATE NOCASE);
CREATE TABLE scan_errors (
    scan_id INTEGER NOT NULL REFERENCES scan_runs(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    message TEXT NOT NULL
);
PRAGMA user_version = 2;
