CREATE TABLE cleanup_plans (
    id INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL,
    state TEXT NOT NULL,
    detail TEXT NOT NULL
);
CREATE TABLE operation_log (
    id INTEGER PRIMARY KEY,
    plan_id INTEGER NOT NULL REFERENCES cleanup_plans(id),
    timestamp INTEGER NOT NULL,
    path TEXT NOT NULL,
    state TEXT NOT NULL,
    message TEXT NOT NULL
);
PRAGMA user_version = 4;
