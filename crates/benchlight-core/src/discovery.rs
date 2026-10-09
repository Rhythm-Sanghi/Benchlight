use benchlight_platform_windows::is_reparse_point;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub path: PathBuf,
    pub name: String,
    pub evidence: Vec<String>,
    pub languages: Vec<String>,
    pub last_activity: Option<i64>,
    pub logical_bytes: i64,
}

pub fn timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

fn marker(name: &str, directory: bool) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        ".git" if directory => Some("Git"),
        ".git" if !directory => Some("Git worktree"),
        "package.json" if !directory => Some("JavaScript / TypeScript"),
        "pyproject.toml" | "requirements.txt" if !directory => Some("Python"),
        "cargo.toml" if !directory => Some("Rust"),
        "go.mod" if !directory => Some("Go"),
        "pom.xml" | "build.gradle" | "build.gradle.kts" if !directory => Some("Java / Kotlin"),
        "composer.json" if !directory => Some("PHP"),
        "gemfile" if !directory => Some("Ruby"),
        _ if !directory && (lower.ends_with(".sln") || lower.ends_with(".csproj")) => Some(".NET"),
        _ => None,
    }
}

pub fn detect_project(path: &Path) -> std::io::Result<Option<Project>> {
    detect_project_with_cancel(path, &AtomicBool::new(false))
}

pub(crate) fn detect_project_with_cancel(
    path: &Path,
    cancel: &AtomicBool,
) -> std::io::Result<Option<Project>> {
    let mut evidence = Vec::new();
    let mut languages = Vec::new();
    let mut last_activity = None;
    for entry in fs::read_dir(path)? {
        if cancel.load(Ordering::Relaxed) {
            return Ok(None);
        }
        let entry = entry?;
        let metadata = fs::symlink_metadata(entry.path())?;
        if is_reparse_point(&metadata) {
            continue;
        }
        if let Ok(modified) = metadata.modified().and_then(|date| {
            date.duration_since(UNIX_EPOCH)
                .map_err(std::io::Error::other)
        }) {
            last_activity = Some(
                last_activity
                    .unwrap_or(0)
                    .max(i64::try_from(modified.as_secs()).unwrap_or(i64::MAX)),
            );
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(language) = marker(&name, metadata.is_dir()) {
            if evidence.len() < 256 {
                evidence.push(name);
            }
            if !language.starts_with("Git") && !languages.iter().any(|value| value == language) {
                languages.push(language.to_owned());
            }
        }
    }
    if evidence.is_empty() {
        return Ok(None);
    }
    evidence.sort();
    languages.sort();
    Ok(Some(Project {
        path: path.to_path_buf(),
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        evidence,
        languages,
        last_activity,
        logical_bytes: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Benchlight;

    #[test]
    fn evidence_is_recorded_and_generic_directories_are_not_projects() {
        let fixture = tempfile::tempdir().unwrap();
        fs::create_dir(fixture.path().join("target")).unwrap();
        assert!(detect_project(fixture.path()).unwrap().is_none());
        fs::write(fixture.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(fixture.path().join("package.json"), "{}").unwrap();
        let project = detect_project(fixture.path()).unwrap().unwrap();
        assert_eq!(project.evidence, ["Cargo.toml", "package.json"]);
        assert_eq!(project.languages.len(), 2);
    }

    #[test]
    fn scan_is_persisted_paged_and_cancelled_without_mutating_sources() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("source");
        fs::create_dir_all(root.join("node/node_modules/dependency")).unwrap();
        fs::write(root.join("node/package.json"), "{}").unwrap();
        fs::write(root.join("node/node_modules/dependency/package.json"), "{}").unwrap();
        let app = Benchlight::open(fixture.path().join("db")).unwrap();
        app.add_root(&root).unwrap();
        let scan = app.scan(&AtomicBool::new(false)).unwrap();
        assert_eq!(scan.projects, 1);
        assert_eq!(app.projects(0, 200).unwrap()[0].evidence, ["package.json"]);
        assert!(app.projects(1, 200).unwrap().is_empty());
        assert!(root.join("node/package.json").exists());
        assert_eq!(app.scan(&AtomicBool::new(true)).unwrap().state, "cancelled");
    }
}
