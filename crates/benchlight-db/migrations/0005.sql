CREATE TABLE tool_observations (
    path TEXT PRIMARY KEY COLLATE NOCASE,
    name TEXT NOT NULL,
    detail TEXT NOT NULL
);
PRAGMA user_version = 5;
