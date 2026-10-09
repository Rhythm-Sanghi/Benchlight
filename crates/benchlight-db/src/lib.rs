pub use rusqlite::Error as DatabaseError;
use rusqlite::{Connection, Result, params};
use std::{path::Path, time::Duration};

pub struct Database(Connection);

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        Self::initialize(Connection::open(path)?)
    }

    fn initialize(mut connection: Connection) -> Result<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 6 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        if version == 0 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/0001.sql"))?;
            transaction.commit()?;
        }
        if version < 2 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/0002.sql"))?;
            transaction.commit()?;
        }
        if version < 3 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/0003.sql"))?;
            transaction.commit()?;
        }
        if version < 4 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/0004.sql"))?;
            transaction.commit()?;
        }
        if version < 5 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/0005.sql"))?;
            transaction.commit()?;
        }
        if version < 6 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(include_str!("../migrations/0006.sql"))?;
            transaction.commit()?;
        }
        Ok(Self(connection))
    }

    pub fn roots(&self) -> Result<Vec<String>> {
        self.0
            .prepare("SELECT path FROM scan_roots WHERE enabled = 1 ORDER BY path")?
            .query_map([], |row| row.get(0))?
            .collect()
    }
    pub fn save_working_snapshot(
        &self,
        project: &str,
        timestamp: i64,
        detail: &str,
    ) -> Result<i64> {
        self.0.execute(
            "INSERT INTO working_snapshots(project,created_at,detail) VALUES (?1,?2,?3)",
            params![project, timestamp, detail],
        )?;
        Ok(self.0.last_insert_rowid())
    }
    pub fn working_snapshot(&self, project: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT detail FROM working_snapshots WHERE project=?1 ORDER BY id DESC LIMIT 1",
                [project],
                |row| row.get(0),
            )
            .optional()
    }
    pub fn save_tools(&self, tools: &[(String, String, String)], errors: &str) -> Result<()> {
        let transaction = self.0.unchecked_transaction()?;
        transaction.execute("DELETE FROM tool_observations", [])?;
        for (path, name, json) in tools {
            transaction.execute(
                "INSERT INTO tool_observations(path,name,detail) VALUES (?1,?2,?3)",
                params![path, name, json],
            )?;
        }
        transaction.execute("INSERT INTO settings(key,value) VALUES ('tool_errors',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[errors])?;
        transaction.commit()
    }
    pub fn tools(&self) -> Result<Vec<String>> {
        self.0
            .prepare("SELECT detail FROM tool_observations ORDER BY name COLLATE NOCASE,path")?
            .query_map([], |row| row.get(0))?
            .collect()
    }

    pub fn create_cleanup_plan(&self, created_at: i64, detail: &str) -> Result<i64> {
        self.0.execute(
            "INSERT INTO cleanup_plans(created_at,state,detail) VALUES (?1,'preview',?2)",
            params![created_at, detail],
        )?;
        Ok(self.0.last_insert_rowid())
    }
    pub fn cleanup_plan(&self, id: i64) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT detail FROM cleanup_plans WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
    }
    pub fn cleanup_plans(&self) -> Result<Vec<String>> {
        self.0
            .prepare("SELECT detail FROM cleanup_plans ORDER BY id DESC LIMIT 100")?
            .query_map([], |row| row.get(0))?
            .collect()
    }
    pub fn update_cleanup_plan(&self, id: i64, state: &str, detail: &str) -> Result<()> {
        self.0.execute(
            "UPDATE cleanup_plans SET state=?2,detail=?3 WHERE id=?1",
            params![id, state, detail],
        )?;
        Ok(())
    }
    pub fn log_operation(
        &self,
        plan: i64,
        timestamp: i64,
        path: &str,
        state: &str,
        message: &str,
    ) -> Result<()> {
        self.0.execute("INSERT INTO operation_log(plan_id,timestamp,path,state,message) VALUES (?1,?2,?3,?4,?5)",params![plan,timestamp,path,state,message])?;
        Ok(())
    }
    pub fn operation_log(&self, plan: i64) -> Result<Vec<OperationEvent>> {
        self.0.prepare("SELECT timestamp,path,state,message FROM operation_log WHERE plan_id=?1 ORDER BY id")?.query_map([plan], |row| Ok(OperationEvent {timestamp:row.get(0)?,path:row.get(1)?,state:row.get(2)?,message:row.get(3)?}))?.collect()
    }

    pub fn add_root(&self, path: &str) -> Result<()> {
        self.0.execute(
            "INSERT INTO scan_roots(path) VALUES (?1) ON CONFLICT(path) DO UPDATE SET enabled = 1",
            [path],
        )?;
        Ok(())
    }

    pub fn remove_root(&self, path: &str) -> Result<()> {
        self.0
            .execute("DELETE FROM scan_roots WHERE path = ?1", [path])?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.0.execute("INSERT INTO settings(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value", params![key, value])?;
        Ok(())
    }

    pub fn begin_scan(&self, timestamp: i64) -> Result<i64> {
        self.0.execute(
            "UPDATE scan_runs SET state = 'interrupted' WHERE state = 'running'",
            [],
        )?;
        self.0.execute(
            "INSERT INTO scan_runs(started_at, state) VALUES (?1, 'running')",
            [timestamp],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn update_scan(&self, scan: &ScanRun) -> Result<()> {
        self.0.execute("UPDATE scan_runs SET state=?2, directories=?3, projects=?4, errors=?5, finished_at=?6, logical_bytes=?7, skipped=?8 WHERE id=?1",
            params![scan.id, scan.state, scan.directories, scan.projects, scan.errors, scan.finished_at, scan.logical_bytes, scan.skipped])?;
        Ok(())
    }

    pub fn latest_scan(&self) -> Result<Option<ScanRun>> {
        use rusqlite::OptionalExtension;
        self.0.query_row("SELECT id, started_at, finished_at, state, directories, projects, errors, logical_bytes, skipped FROM scan_runs ORDER BY id DESC LIMIT 1", [], |row| Ok(ScanRun {
            id: row.get(0)?, started_at: row.get(1)?, finished_at: row.get(2)?, state: row.get(3)?, directories: row.get(4)?, projects: row.get(5)?, errors: row.get(6)?, logical_bytes: row.get(7)?, skipped: row.get(8)?,
        })).optional()
    }

    pub fn save_project(&self, scan_id: i64, project: &ProjectRecord) -> Result<bool> {
        let inserted = self.0.execute("INSERT OR IGNORE INTO projects(scan_id, path, name, evidence, languages, last_activity) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![scan_id, project.path, project.name, project.evidence, project.languages, project.last_activity])?;
        Ok(inserted == 1)
    }

    pub fn projects(&self, scan_id: i64, offset: u32, limit: u32) -> Result<Vec<ProjectRecord>> {
        self.projects_sorted(scan_id, offset, limit, "name", false)
    }
    pub fn projects_sorted(
        &self,
        scan_id: i64,
        offset: u32,
        limit: u32,
        sort: &str,
        descending: bool,
    ) -> Result<Vec<ProjectRecord>> {
        let column = match sort {
            "name" => "name COLLATE NOCASE",
            "stack" => "languages COLLATE NOCASE",
            "size" => "logical_bytes",
            "activity" => "last_activity",
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        let direction = if descending { "DESC" } else { "ASC" };
        let query = format!(
            "SELECT path, name, evidence, languages, last_activity, logical_bytes FROM projects WHERE scan_id=?1 ORDER BY {column} {direction}, path LIMIT ?2 OFFSET ?3"
        );
        self.0
            .prepare(&query)?
            .query_map(params![scan_id, limit.min(200), offset], |row| {
                Ok(ProjectRecord {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    evidence: row.get(2)?,
                    languages: row.get(3)?,
                    last_activity: row.get(4)?,
                    logical_bytes: row.get(5)?,
                })
            })?
            .collect()
    }

    pub fn contains_project(&self, scan_id: i64, path: &str) -> Result<bool> {
        self.0.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE scan_id=?1 AND path=?2)",
            params![scan_id, path],
            |row| row.get(0),
        )
    }

    pub fn set_project_size(&self, scan_id: i64, path: &str, bytes: i64) -> Result<()> {
        self.0.execute(
            "UPDATE projects SET logical_bytes=?3 WHERE scan_id=?1 AND path=?2",
            params![scan_id, path, bytes],
        )?;
        Ok(())
    }

    pub fn save_candidate(
        &self,
        scan_id: i64,
        path: &str,
        category: &str,
        classification: &str,
        bytes: i64,
        detail: &str,
    ) -> Result<()> {
        self.0.execute("INSERT OR REPLACE INTO storage_candidates(scan_id, path, category, classification, logical_bytes, detail) VALUES (?1,?2,?3,?4,?5,?6)", params![scan_id, path, category, classification, bytes, detail])?;
        Ok(())
    }

    pub fn candidates(&self, scan_id: i64, offset: u32, limit: u32) -> Result<Vec<String>> {
        self.candidates_sorted(scan_id, offset, limit, "size", true)
    }
    pub fn candidates_sorted(
        &self,
        scan_id: i64,
        offset: u32,
        limit: u32,
        sort: &str,
        descending: bool,
    ) -> Result<Vec<String>> {
        let column = match sort {
            "path" => "path COLLATE NOCASE",
            "size" => "logical_bytes",
            "classification" => "classification",
            _ => return Err(rusqlite::Error::InvalidQuery),
        };
        let direction = if descending { "DESC" } else { "ASC" };
        let query = format!(
            "SELECT detail FROM storage_candidates WHERE scan_id=?1 ORDER BY {column} {direction}, path LIMIT ?2 OFFSET ?3"
        );
        self.0
            .prepare(&query)?
            .query_map(params![scan_id, limit.min(200), offset], |row| row.get(0))?
            .collect()
    }

    pub fn candidate(&self, scan_id: i64, path: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT detail FROM storage_candidates WHERE scan_id=?1 AND path=?2",
                params![scan_id, path],
                |row| row.get(0),
            )
            .optional()
    }

    pub fn category_totals(&self, scan_id: i64) -> Result<Vec<CategoryTotal>> {
        self.0.prepare("SELECT category, classification, SUM(logical_bytes), COUNT(*) FROM storage_candidates WHERE scan_id=?1 GROUP BY category, classification ORDER BY SUM(logical_bytes) DESC")?
            .query_map([scan_id], |row| Ok(CategoryTotal { category: row.get(0)?, classification: row.get(1)?, logical_bytes: row.get(2)?, items: row.get(3)? }))?.collect()
    }

    pub fn add_scan_error(&self, scan_id: i64, path: &str, message: &str) -> Result<()> {
        self.0.execute(
            "INSERT INTO scan_errors(scan_id, path, message) VALUES (?1, ?2, ?3)",
            params![scan_id, path, message],
        )?;
        Ok(())
    }

    pub fn scan_errors(&self, scan_id: i64) -> Result<Vec<ScanError>> {
        self.0
            .prepare("SELECT path, message FROM scan_errors WHERE scan_id=?1 LIMIT 500")?
            .query_map([scan_id], |row| {
                Ok(ScanError {
                    path: row.get(0)?,
                    message: row.get(1)?,
                })
            })?
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanRun {
    pub id: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub state: String,
    pub directories: i64,
    pub projects: i64,
    pub errors: i64,
    pub logical_bytes: i64,
    pub skipped: i64,
}

pub struct ProjectRecord {
    pub path: String,
    pub name: String,
    pub evidence: String,
    pub languages: String,
    pub last_activity: Option<i64>,
    pub logical_bytes: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct CategoryTotal {
    pub category: String,
    pub classification: String,
    pub logical_bytes: i64,
    pub items: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct ScanError {
    pub path: String,
    pub message: String,
}
#[derive(Debug, serde::Serialize)]
pub struct OperationEvent {
    pub timestamp: i64,
    pub path: String,
    pub state: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_and_roots_are_idempotent() {
        let db = Database::initialize(Connection::open_in_memory().unwrap()).unwrap();
        db.add_root(r"C:\source").unwrap();
        db.add_root(r"c:\SOURCE").unwrap();
        assert_eq!(db.roots().unwrap().len(), 1);
        db.remove_root(r"C:\source").unwrap();
        assert!(db.roots().unwrap().is_empty());
        db.set_setting("theme", "dark").unwrap();
        db.set_setting("theme", "light").unwrap();
        assert_eq!(db.setting("theme").unwrap().as_deref(), Some("light"));
    }

    #[test]
    fn newer_schema_is_rejected() {
        let connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 999).unwrap();
        assert!(Database::initialize(connection).is_err());
    }
    #[test]
    fn sorting_happens_before_pagination_and_rejects_unknown_columns() {
        let db = Database::initialize(Connection::open_in_memory().unwrap()).unwrap();
        let scan = db.begin_scan(1).unwrap();
        for index in 0..205 {
            let path = format!("C:\\projects\\{index:03}");
            db.save_project(
                scan,
                &ProjectRecord {
                    path: path.clone(),
                    name: format!("Project {index:03}"),
                    evidence: "[]".into(),
                    languages: "[]".into(),
                    last_activity: Some(index),
                    logical_bytes: 0,
                },
            )
            .unwrap();
            db.set_project_size(scan, &path, index).unwrap();
        }
        assert_eq!(
            db.projects_sorted(scan, 0, 100, "size", true).unwrap()[0].logical_bytes,
            204
        );
        assert_eq!(
            db.projects_sorted(scan, 200, 100, "size", true)
                .unwrap()
                .len(),
            5
        );
        assert!(
            db.projects_sorted(scan, 0, 100, "name; DROP TABLE projects", false)
                .is_err()
        );
    }

    #[test]
    fn migration_preserves_existing_roots() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001.sql"))
            .unwrap();
        connection
            .execute("INSERT INTO scan_roots(path) VALUES ('C:\\source')", [])
            .unwrap();
        let database = Database::initialize(connection).unwrap();
        assert_eq!(database.roots().unwrap(), ["C:\\source"]);
        assert!(database.latest_scan().unwrap().is_none());
    }
}
