use benchlight_platform_windows::is_reparse_point;
use serde::{Deserialize, Serialize};
use std::io::Read;
pub mod fingerprint;
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Classification {
    Protected,
    Review,
    Rebuildable,
    Cache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub path: PathBuf,
    pub project: Option<PathBuf>,
    pub category: String,
    pub classification: Classification,
    pub reason: String,
    pub evidence: Vec<String>,
    pub recreate: Option<String>,
    pub logical_bytes: u64,
    pub files: u64,
    pub complete: bool,
    pub modified: Option<u64>,
    #[serde(default)]
    pub identity: Option<benchlight_platform_windows::cleanup::Identity>,
}

fn declared_dependencies(project: &Path) -> bool {
    let Ok(file) = fs::File::open(project.join("package.json")) else {
        return false;
    };
    let mut contents = Vec::new();
    if file.take(1_048_577).read_to_end(&mut contents).is_err() || contents.len() > 1_048_576 {
        return false;
    }
    let Ok(manifest) = serde_json::from_slice::<serde_json::Value>(&contents) else {
        return false;
    };
    ["dependencies", "devDependencies", "optionalDependencies"]
        .iter()
        .any(|key| {
            manifest
                .get(key)
                .and_then(|value| value.as_object())
                .is_some_and(|value| !value.is_empty())
        })
        || manifest.get("workspaces").is_some_and(|value| {
            value.as_array().is_some_and(|items| !items.is_empty())
                || value
                    .get("packages")
                    .and_then(|items| items.as_array())
                    .is_some_and(|items| !items.is_empty())
        })
}
fn cargo_cache_marker(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path.join("CACHEDIR.TAG")) else {
        return false;
    };
    let mut signature = [0; 43];
    file.read_exact(&mut signature).is_ok()
        && &signature == b"Signature: 8a477f597d28d172789f06886806bc55"
}

pub fn classify_candidate(path: &Path, project: &Path, evidence: &[String]) -> Option<Candidate> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let has = |marker: &str| {
        evidence
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(marker))
    };
    let dotnet = evidence
        .iter()
        .any(|entry| entry.to_ascii_lowercase().ends_with(".csproj"));
    let (category, classification, reason, recreate) = match name.as_str() {
        ".git" if has(".git") => (
            "Git metadata",
            Classification::Protected,
            "Repository history and configuration are protected.",
            None,
        ),
        "node_modules" if has("package.json") && declared_dependencies(project) => (
            "Node dependencies",
            Classification::Rebuildable,
            "Manifest declares dependencies/workspaces; local changes would be lost.",
            Some(if project.join("package-lock.json").is_file() {
                "npm ci"
            } else if project.join("pnpm-lock.yaml").is_file() {
                "pnpm install --frozen-lockfile"
            } else if project.join("yarn.lock").is_file() {
                "yarn install --immutable"
            } else {
                "Install dependencies with the project's package manager; versions may change without a lockfile."
            }),
        ),
        "node_modules" if has("package.json") => (
            "Node dependencies",
            Classification::Review,
            "No declared dependencies could be established from package.json. Contents may not be reproducible.",
            None,
        ),
        ".venv" | "venv"
            if (has("pyproject.toml") || has("requirements.txt"))
                && path.join("pyvenv.cfg").is_file() =>
        {
            (
                "Python environments",
                Classification::Review,
                "Python environment with pyvenv.cfg; it may contain packages or data not declared by the project.",
                None,
            )
        }
        "target" if has("Cargo.toml") && cargo_cache_marker(path) => (
            "Rust build output",
            Classification::Rebuildable,
            "Cargo manifest and Cargo target cache marker found.",
            Some("cargo build"),
        ),
        "target" if has("pom.xml") && path.join("maven-status").is_dir() => (
            "Maven build output",
            Classification::Rebuildable,
            "Maven manifest and maven-status build metadata found.",
            Some("mvn package"),
        ),
        "obj" if dotnet && path.join("project.assets.json").is_file() => (
            ".NET build output",
            Classification::Rebuildable,
            ".NET project and NuGet restore metadata found.",
            Some("dotnet build"),
        ),
        "bin" if dotnet => (
            ".NET build output",
            Classification::Review,
            "Directory beside a .NET project; ownership of its contents needs review.",
            None,
        ),
        ".gradle" if has("build.gradle") || has("build.gradle.kts") => (
            "Gradle project state",
            Classification::Review,
            "Gradle manifest found; project state can include local configuration.",
            None,
        ),
        "build" if has("build.gradle") || has("build.gradle.kts") => (
            "Gradle build output",
            Classification::Review,
            "Gradle project found; this generic directory needs content review.",
            None,
        ),
        "dist" | "build" | "coverage" if has("package.json") => (
            "Project output",
            Classification::Review,
            "Project manifest found; a generic output name does not establish ownership.",
            None,
        ),
        _ => return None,
    };
    Some(Candidate {
        path: path.to_path_buf(),
        project: Some(project.to_path_buf()),
        category: category.into(),
        classification,
        reason: reason.into(),
        evidence: evidence.to_vec(),
        recreate: recreate.map(str::to_owned),
        logical_bytes: 0,
        files: 0,
        complete: false,
        modified: None,
        identity: None,
    })
}

#[derive(Debug, Default, Serialize)]
pub struct Measurement {
    pub logical_bytes: u64,
    pub files: u64,
    pub skipped: u64,
    pub errors: u64,
    pub cancelled: bool,
}

pub fn measure_directory(
    path: &Path,
    cancel: &AtomicBool,
    error: impl FnMut(&Path, &io::Error),
    progress: impl FnMut(&Measurement),
) -> Measurement {
    measure_directory_excluding(path, cancel, &[], error, progress)
}

pub fn measure_directory_excluding(
    path: &Path,
    cancel: &AtomicBool,
    exclusions: &[PathBuf],
    mut error: impl FnMut(&Path, &io::Error),
    mut progress: impl FnMut(&Measurement),
) -> Measurement {
    let mut result = Measurement::default();
    if let Err(failure) = benchlight_platform_windows::ensure_no_reparse_path(path) {
        error(path, &failure);
        result.errors = 1;
        return result;
    }
    let mut pending = Some(path.to_path_buf());
    let mut frames: Vec<(fs::ReadDir, PathBuf, std::time::SystemTime)> = Vec::new();
    loop {
        if cancel.load(Ordering::Relaxed) {
            result.cancelled = true;
            break;
        }
        let next = pending.take().or_else(|| {
            while let Some((entries, parent, before)) = frames.last_mut() {
                match entries.next() {
                    Some(Ok(entry)) => return Some(entry.path()),
                    Some(Err(failure)) => {
                        result.errors += 1;
                        error(parent, &failure);
                    }
                    None => {
                        if fs::symlink_metadata(&*parent)
                            .and_then(|metadata| metadata.modified())
                            .ok()
                            .as_ref()
                            != Some(before)
                        {
                            result.errors += 1;
                            error(
                                parent,
                                &io::Error::other(
                                    "Directory changed while it was measured; size is partial.",
                                ),
                            );
                        }
                        frames.pop();
                    }
                }
            }
            None
        });
        let Some(path) = next else { break };
        if exclusions
            .iter()
            .any(|excluded| benchlight_platform_windows::contains_path(excluded, &path))
        {
            result.skipped += 1;
            continue;
        }
        let inspected = (|| -> io::Result<()> {
            let metadata = fs::symlink_metadata(&path)?;
            if is_reparse_point(&metadata) {
                result.skipped += 1;
                return Ok(());
            }
            if metadata.is_file() {
                result.logical_bytes = result.logical_bytes.saturating_add(metadata.len());
                result.files += 1;
            } else if metadata.is_dir() {
                if frames.len() >= 128 {
                    return Err(io::Error::other("Directory depth limit reached."));
                }
                frames.push((fs::read_dir(&path)?, path.clone(), metadata.modified()?));
            }
            Ok(())
        })();
        if let Err(failure) = inspected {
            result.errors += 1;
            error(&path, &failure);
        }
        progress(&result);
    }
    result
}

pub fn modified(path: &Path) -> Option<u64> {
    fs::symlink_metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|value| value.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_and_invalid_node_manifests_need_review() {
        let root = tempfile::tempdir().unwrap();
        let dependencies = root.path().join("node_modules");
        fs::create_dir(&dependencies).unwrap();
        for contents in ["{}", "{\"workspaces\":[]}", "invalid"] {
            fs::write(root.path().join("package.json"), contents).unwrap();
            assert_eq!(
                classify_candidate(&dependencies, root.path(), &["package.json".into()])
                    .unwrap()
                    .classification,
                Classification::Review
            );
        }
        fs::write(
            root.path().join("package.json"),
            r#"{"dependencies":{"react":"19"}}"#,
        )
        .unwrap();
        assert_eq!(
            classify_candidate(&dependencies, root.path(), &["package.json".into()])
                .unwrap()
                .classification,
            Classification::Rebuildable
        );
    }
    #[test]
    fn generic_names_require_manifest_and_specific_build_evidence() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        fs::create_dir(&target).unwrap();
        assert!(classify_candidate(&target, root.path(), &[]).is_none());
        assert!(classify_candidate(&target, root.path(), &["Cargo.toml".into()]).is_none());
        fs::write(target.join("CACHEDIR.TAG"), "fixture").unwrap();
        assert!(classify_candidate(&target, root.path(), &["Cargo.toml".into()]).is_none());
        fs::write(
            target.join("CACHEDIR.TAG"),
            "Signature: 8a477f597d28d172789f06886806bc55",
        )
        .unwrap();
        assert_eq!(
            classify_candidate(&target, root.path(), &["Cargo.toml".into()])
                .unwrap()
                .classification,
            Classification::Rebuildable
        );
    }
    #[test]
    fn measurement_is_exact_for_regular_files_and_can_cancel() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("hidden")).unwrap();
        fs::write(root.path().join("one"), [0; 7]).unwrap();
        fs::write(root.path().join("hidden/two"), [0; 11]).unwrap();
        let size = measure_directory(
            root.path(),
            &AtomicBool::new(false),
            |_, _| panic!("unexpected read error"),
            |_| {},
        );
        assert_eq!(size.logical_bytes, 18);
        assert_eq!(size.files, 2);
        assert!(
            measure_directory(root.path(), &AtomicBool::new(true), |_, _| {}, |_| {}).cancelled
        );
    }
    #[test]
    fn changing_directory_membership_is_reported_as_partial() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("one"), [0; 7]).unwrap();
        let mut changed = false;
        let measurement = measure_directory(
            root.path(),
            &AtomicBool::new(false),
            |_, _| {},
            |progress| {
                if progress.files == 1 && !changed {
                    fs::write(root.path().join("new"), [0; 3]).unwrap();
                    changed = true;
                }
            },
        );
        assert!(changed);
        assert!(measurement.errors > 0);
    }
}
