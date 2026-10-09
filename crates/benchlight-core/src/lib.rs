use benchlight_db::Database;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
mod cleanup;
mod logs;
pub mod snapshots;
pub mod tools;
pub use tools::{Tool, ToolReport};
pub mod discovery;
mod scanning;
pub use benchlight_db::CategoryTotal;
pub use benchlight_db::{ScanError, ScanRun};
pub use benchlight_storage::{Candidate, Classification};
pub use cleanup::{CleanupItem, CleanupPlan};
pub use discovery::Project;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Filesystem operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Local database operation failed: {0}")]
    Database(#[from] benchlight_db::DatabaseError),
    #[error("{0}")]
    InvalidRoot(String),
    #[error("Local metadata could not be decoded: {0}")]
    Metadata(#[from] serde_json::Error),
}

#[derive(Serialize)]
pub struct AppStatus {
    pub version: &'static str,
    pub data_directory: PathBuf,
    pub roots: Vec<String>,
    pub suggested_roots: Vec<PathBuf>,
    pub theme: String,
    pub scan: Option<ScanRun>,
    pub include_caches: bool,
    pub exclusions: Vec<String>,
}

pub struct Benchlight {
    database: Database,
    data_directory: PathBuf,
}

impl Benchlight {
    pub fn open(data_directory: PathBuf) -> Result<Self, Error> {
        fs::create_dir_all(&data_directory)?;
        let data_directory = fs::canonicalize(data_directory)?;
        let database = Database::open(&data_directory.join("benchlight.db"))?;
        Ok(Self {
            database,
            data_directory,
        })
    }

    pub fn open_default() -> Result<Self, Error> {
        Self::open(benchlight_platform_windows::data_directory()?)
    }

    pub fn status(&self) -> Result<AppStatus, Error> {
        Ok(AppStatus {
            version: env!("CARGO_PKG_VERSION"),
            data_directory: self.data_directory.clone(),
            roots: self.database.roots()?,
            suggested_roots: benchlight_platform_windows::suggested_roots(),
            theme: self
                .database
                .setting("theme")?
                .unwrap_or_else(|| "system".into()),
            scan: self.database.latest_scan()?,
            include_caches: self.database.setting("include_caches")?.as_deref() == Some("true"),
            exclusions: self
                .database
                .setting("exclusions")?
                .map(|json| serde_json::from_str(&json))
                .transpose()?
                .unwrap_or_default(),
        })
    }

    pub fn add_root(&self, path: &Path) -> Result<(), Error> {
        if !path.is_absolute() {
            return Err(Error::InvalidRoot("Choose an absolute folder path.".into()));
        }
        benchlight_platform_windows::ensure_local_root(path)?;
        let metadata = fs::symlink_metadata(path)?;
        for ancestor in path.ancestors() {
            if benchlight_platform_windows::is_reparse_point(&fs::symlink_metadata(ancestor)?) {
                return Err(Error::InvalidRoot(
                    "The folder's path contains a link, junction or cloud placeholder.".into(),
                ));
            }
        }
        if !metadata.is_dir() || benchlight_platform_windows::is_reparse_point(&metadata) {
            return Err(Error::InvalidRoot(
                "Choose a regular directory, not a link, junction or cloud placeholder.".into(),
            ));
        }
        let path = fs::canonicalize(path)?;
        let path = path.to_str().ok_or_else(|| {
            Error::InvalidRoot("This path cannot be represented as Unicode.".into())
        })?;
        self.database.add_root(path)?;
        Ok(())
    }

    pub fn remove_root(&self, path: &str) -> Result<(), Error> {
        self.database.remove_root(path)?;
        Ok(())
    }

    pub fn set_theme(&self, theme: &str) -> Result<(), Error> {
        if !["system", "light", "dark"].contains(&theme) {
            return Err(Error::InvalidRoot("Unknown theme.".into()));
        }
        self.database.set_setting("theme", theme)?;
        Ok(())
    }

    pub fn projects(&self, offset: u32, limit: u32) -> Result<Vec<Project>, Error> {
        self.projects_sorted(offset, limit, "name", false)
    }
    pub fn projects_sorted(
        &self,
        offset: u32,
        limit: u32,
        sort: &str,
        descending: bool,
    ) -> Result<Vec<Project>, Error> {
        let Some(scan) = self.database.latest_scan()? else {
            return Ok(Vec::new());
        };
        self.database
            .projects_sorted(scan.id, offset, limit, sort, descending)?
            .into_iter()
            .map(|row| {
                Ok(Project {
                    path: row.path.into(),
                    name: row.name,
                    evidence: serde_json::from_str(&row.evidence)?,
                    languages: serde_json::from_str(&row.languages)?,
                    last_activity: row.last_activity,
                    logical_bytes: row.logical_bytes,
                })
            })
            .collect()
    }

    pub fn scan_errors(&self) -> Result<Vec<ScanError>, Error> {
        match self.database.latest_scan()? {
            Some(scan) => Ok(self.database.scan_errors(scan.id)?),
            None => Ok(Vec::new()),
        }
    }

    pub fn open_project_folder(&self, path: &Path) -> Result<(), Error> {
        let scan = self
            .database
            .latest_scan()?
            .ok_or_else(|| Error::InvalidRoot("Scan projects first.".into()))?;
        if !self
            .database
            .contains_project(scan.id, &path.to_string_lossy())?
        {
            return Err(Error::InvalidRoot(
                "The folder is not in the current project scan.".into(),
            ));
        }
        for ancestor in path.ancestors() {
            if benchlight_platform_windows::is_reparse_point(&fs::symlink_metadata(ancestor)?) {
                return Err(Error::InvalidRoot(
                    "The folder now contains a link or junction. Scan again.".into(),
                ));
            }
        }
        benchlight_platform_windows::open_folder(path)?;
        Ok(())
    }

    pub fn set_include_caches(&self, enabled: bool) -> Result<(), Error> {
        self.database
            .set_setting("include_caches", if enabled { "true" } else { "false" })?;
        Ok(())
    }

    pub fn candidates(&self, offset: u32, limit: u32) -> Result<Vec<Candidate>, Error> {
        self.candidates_sorted(offset, limit, "size", true)
    }
    pub fn candidates_sorted(
        &self,
        offset: u32,
        limit: u32,
        sort: &str,
        descending: bool,
    ) -> Result<Vec<Candidate>, Error> {
        let Some(scan) = self.database.latest_scan()? else {
            return Ok(Vec::new());
        };
        self.database
            .candidates_sorted(scan.id, offset, limit, sort, descending)?
            .into_iter()
            .map(|json| Ok(serde_json::from_str(&json)?))
            .collect()
    }

    pub fn set_exclusions(&self, paths: &[String]) -> Result<(), Error> {
        if paths.len() > 200
            || paths.iter().any(|path| {
                !Path::new(path).is_absolute()
                    || Path::new(path)
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
            })
        {
            return Err(Error::InvalidRoot(
                "Exclusions must be absolute paths without parent traversal (maximum 200).".into(),
            ));
        }
        self.database
            .set_setting("exclusions", &serde_json::to_string(paths)?)?;
        Ok(())
    }

    pub fn category_totals(&self) -> Result<Vec<CategoryTotal>, Error> {
        match self.database.latest_scan()? {
            Some(scan) => Ok(self.database.category_totals(scan.id)?),
            None => Ok(Vec::new()),
        }
    }

    pub fn candidate(&self, path: &Path) -> Result<Candidate, Error> {
        let path = fs::canonicalize(path)?;
        let scan = self
            .database
            .latest_scan()?
            .ok_or_else(|| Error::InvalidRoot("Scan storage first.".into()))?;
        let json = self
            .database
            .candidate(scan.id, &path.to_string_lossy())?
            .ok_or_else(|| {
                Error::InvalidRoot("The directory is not in the latest storage scan.".into())
            })?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn open_storage_folder(&self, path: &Path) -> Result<(), Error> {
        self.candidate(path)?;
        benchlight_platform_windows::ensure_no_reparse_path(path)?;
        let folder = if path.is_file() {
            path.parent()
                .ok_or_else(|| Error::InvalidRoot("No containing folder.".into()))?
        } else {
            path
        };
        benchlight_platform_windows::open_folder(folder)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_persist_without_scanning_on_launch() {
        let temporary = tempfile::tempdir().unwrap();
        let data = temporary.path().join("data");
        let root = temporary.path().join("projects");
        fs::create_dir(&root).unwrap();
        let app = Benchlight::open(data.clone()).unwrap();
        assert!(app.status().unwrap().roots.is_empty());
        app.add_root(&root).unwrap();
        app.add_root(&root).unwrap();
        drop(app);
        let app = Benchlight::open(data).unwrap();
        assert_eq!(app.status().unwrap().roots.len(), 1);
        assert!(app.add_root(Path::new("..\\projects")).is_err());
    }
}
