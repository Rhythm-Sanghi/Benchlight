ALTER TABLE projects ADD COLUMN logical_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_runs ADD COLUMN logical_bytes INTEGER NOT NULL DEFAULT 0;
ALTER TABLE scan_runs ADD COLUMN skipped INTEGER NOT NULL DEFAULT 0;
CREATE TABLE storage_candidates (
    scan_id INTEGER NOT NULL REFERENCES scan_runs(id) ON DELETE CASCADE,
    path TEXT NOT NULL COLLATE NOCASE,
    category TEXT NOT NULL,
    classification TEXT NOT NULL,
    logical_bytes INTEGER NOT NULL,
    detail TEXT NOT NULL,
    PRIMARY KEY(scan_id, path)
);
CREATE INDEX candidates_size ON storage_candidates(scan_id, logical_bytes DESC);
PRAGMA user_version = 3;
