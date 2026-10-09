use crate::{Benchlight, Error, discovery::timestamp, tools::Tool};
use benchlight_platform_windows::{ensure_no_reparse_path, process::run_stdout_command};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Duration,
};

const LOCKFILES: &[&str] = &[
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
    "Cargo.lock",
    "poetry.lock",
    "uv.lock",
    "Pipfile.lock",
    "composer.lock",
    "Gemfile.lock",
    "packages.lock.json",
    "gradle.lockfile",
    "go.sum",
];
const ENVIRONMENT: &[&str] = &[
    "NODE_ENV",
    "VIRTUAL_ENV",
    "JAVA_HOME",
    "RUSTUP_TOOLCHAIN",
    "DATABASE_URL",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Observation {
    pub value: Option<String>,
    pub error: Option<String>,
}
impl Observation {
    fn known(value: impl Into<String>) -> Self {
        Self {
            value: Some(value.into()),
            error: None,
        }
    }
    fn absent() -> Self {
        Self {
            value: None,
            error: None,
        }
    }
    fn unknown(message: impl Into<String>) -> Self {
        Self {
            value: None,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConstraint {
    pub tool: String,
    pub source: String,
    pub declared: String,
    pub current: Option<String>,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub project: PathBuf,
    pub created_at: i64,
    pub observations: BTreeMap<String, Observation>,
    pub constraints: Vec<RuntimeConstraint>,
    pub errors: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Change {
    pub field: String,
    pub previous: Observation,
    pub current: Observation,
    pub state: String,
}
#[derive(Debug, Serialize)]
pub struct Comparison {
    pub previous: Snapshot,
    pub current: Snapshot,
    pub changes: Vec<Change>,
}

impl Benchlight {
    fn snapshot_project(&self, path: &Path) -> Result<PathBuf, Error> {
        ensure_no_reparse_path(path)?;
        let path = fs::canonicalize(path)?;
        let scan = self
            .database
            .latest_scan()?
            .ok_or_else(|| Error::InvalidRoot("Scan projects first.".into()))?;
        if !self
            .database
            .contains_project(scan.id, &path.to_string_lossy())?
        {
            return Err(Error::InvalidRoot(
                "Choose a project from the latest scan.".into(),
            ));
        }
        Ok(path)
    }

    pub fn working_snapshot(&self, path: &Path) -> Result<Option<Snapshot>, Error> {
        let path = self.snapshot_project(path)?;
        self.database
            .working_snapshot(&path.to_string_lossy())?
            .map(|json| Ok(serde_json::from_str(&json)?))
            .transpose()
    }
    pub fn mark_working(&self, path: &Path) -> Result<Snapshot, Error> {
        let snapshot = self.capture_snapshot(path)?;
        self.save_working(&snapshot)?;
        Ok(snapshot)
    }

    pub fn mark_working_with_health(
        &self,
        path: &Path,
        executable: &Path,
        arguments: &[String],
    ) -> Result<Snapshot, Error> {
        let project = self.snapshot_project(path)?;
        if !executable.is_absolute()
            || !executable
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
            || arguments.len() > 100
            || arguments.iter().any(|argument| argument.len() > 4096)
        {
            return Err(Error::InvalidRoot("A health command must name an absolute native .exe and at most 100 bounded arguments; shell wrappers are not accepted.".into()));
        }
        ensure_no_reparse_path(executable)?;
        let arguments: Vec<_> = arguments.iter().map(String::as_str).collect();
        run_stdout_command(executable, &arguments, &project, Duration::from_secs(120)).map_err(
            |error| {
                Error::InvalidRoot(format!(
                    "Health command failed; no working state was recorded: {error}"
                ))
            },
        )?;
        let mut snapshot = self.capture_snapshot(&project)?;
        snapshot.observations.insert(
            "Health / explicit command".into(),
            Observation::known("Succeeded before capture (output and arguments not stored)"),
        );
        self.save_working(&snapshot)?;
        Ok(snapshot)
    }

    fn save_working(&self, snapshot: &Snapshot) -> Result<(), Error> {
        self.database.save_working_snapshot(
            &snapshot.project.to_string_lossy(),
            snapshot.created_at,
            &serde_json::to_string(&snapshot)?,
        )?;
        Ok(())
    }
    pub fn compare_working(&self, path: &Path) -> Result<Comparison, Error> {
        let previous = self.working_snapshot(path)?.ok_or_else(|| {
            Error::InvalidRoot(
                "No working state recorded. Mark this project as working when you have checked it."
                    .into(),
            )
        })?;
        let mut current = self.capture_snapshot(path)?;
        if previous
            .observations
            .contains_key("Health / explicit command")
        {
            current.observations.insert(
                "Health / explicit command".into(),
                Observation::unknown(
                    "Not run during comparison. Health commands require an explicit invocation.",
                ),
            );
        }
        let changes = compare_snapshots(&previous, &current);
        Ok(Comparison {
            previous,
            current,
            changes,
        })
    }
    pub fn capture_snapshot(&self, path: &Path) -> Result<Snapshot, Error> {
        let project = self.snapshot_project(path)?;
        let report = self.refresh_tools()?;
        let mut snapshot = capture_files(&project);
        snapshot.errors.extend(report.errors);
        for tool in report.tools.iter().filter(|tool| tool.active) {
            snapshot.observations.insert(
                format!("Tool / {} / path", tool.name),
                Observation::known(tool.executable.to_string_lossy()),
            );
            snapshot.observations.insert(
                format!("Tool / {} / version", tool.name),
                match &tool.version {
                    Some(version) => Observation::known(version),
                    None => Observation::unknown(
                        tool.error.as_deref().unwrap_or("Version unavailable."),
                    ),
                },
            );
        }
        snapshot.constraints = runtime_constraints(&project, &report.tools, &mut snapshot.errors);
        for (tool, manifest) in [
            ("node", "package.json"),
            ("npm", "package.json"),
            ("python", "pyproject.toml"),
            ("rustc", "Cargo.toml"),
        ] {
            let observation = if let Some(constraint) = snapshot
                .constraints
                .iter()
                .find(|constraint| constraint.tool == tool)
            {
                Observation::known(&constraint.declared)
            } else if snapshot.errors.iter().any(|error| error.contains(manifest)) {
                Observation::unknown("Runtime declaration could not be interpreted.")
            } else {
                Observation::absent()
            };
            snapshot
                .observations
                .insert(format!("Declared range / {tool}"), observation);
        }
        capture_git(&project, &self.data_directory, &report.tools, &mut snapshot);
        for name in ENVIRONMENT {
            // Presence comes from Benchlight's own process, not a project's shell or .env file.
            snapshot.observations.insert(
                format!("Environment / {name}"),
                Observation::known(if std::env::var_os(name).is_some() {
                    "Present"
                } else {
                    "Missing"
                }),
            );
        }
        Ok(snapshot)
    }
}

pub fn compare_snapshots(previous: &Snapshot, current: &Snapshot) -> Vec<Change> {
    let fields: BTreeSet<_> = previous
        .observations
        .keys()
        .chain(current.observations.keys())
        .collect();
    fields
        .into_iter()
        .map(|field| {
            let previous = previous
                .observations
                .get(field)
                .cloned()
                .unwrap_or_else(Observation::absent);
            let current = current
                .observations
                .get(field)
                .cloned()
                .unwrap_or_else(Observation::absent);
            let state = if previous.error.is_some() || current.error.is_some() {
                "Unknown"
            } else if previous.value == current.value {
                "Unchanged"
            } else {
                "Changed"
            };
            Change {
                field: field.clone(),
                previous,
                current,
                state: state.into(),
            }
        })
        .collect()
}

fn read_file(path: &Path, limit: u64) -> io::Result<Option<Vec<u8>>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    ensure_no_reparse_path(path)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(io::Error::other(
            "Not a regular file or exceeds the inspection limit.",
        ));
    }
    // Deny writers/deleters while reading. This also rejects files currently being replaced.
    use std::os::windows::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new().read(true).share_mode(1).open(path)?;
    let before = file.metadata()?;
    let mut bytes = Vec::new();
    (&mut file).take(limit + 1).read_to_end(&mut bytes)?;
    let after = file.metadata()?;
    if bytes.len() as u64 > limit
        || before.len() != after.len()
        || before.modified()? != after.modified()?
    {
        return Err(io::Error::other("File changed during inspection."));
    }
    Ok(Some(bytes))
}
fn capture_files(project: &Path) -> Snapshot {
    let mut observations = BTreeMap::new();
    for name in LOCKFILES {
        let observation = match read_file(&project.join(name), 32 * 1024 * 1024) {
            Ok(Some(bytes)) => Observation::known(
                Sha256::digest(bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>(),
            ),
            Ok(None) => Observation::absent(),
            Err(error) => Observation::unknown(error.to_string()),
        };
        observations.insert(format!("Lockfile / {name}"), observation);
    }
    Snapshot {
        project: project.into(),
        created_at: timestamp(),
        observations,
        constraints: Vec::new(),
        errors: Vec::new(),
    }
}

fn capture_git(project: &Path, neutral: &Path, tools: &[Tool], snapshot: &mut Snapshot) {
    if !project.join(".git").exists() {
        return;
    }
    let Some(git) = tools.iter().find(|tool| {
        tool.id == "git"
            && tool.active
            && tool
                .executable
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    }) else {
        for name in ["branch", "commit", "tracked worktree"] {
            snapshot.observations.insert(
                format!("Git / {name}"),
                Observation::unknown("Native Git executable not available on PATH."),
            );
        }
        return;
    };
    for (name, args) in [
        ("branch", vec!["symbolic-ref", "--quiet", "--short", "HEAD"]),
        ("commit", vec!["rev-parse", "--verify", "HEAD"]),
        (
            "tracked worktree",
            vec![
                "status",
                "--porcelain=v1",
                "--untracked-files=no",
                "--ignore-submodules=all",
            ],
        ),
    ] {
        let path = project.to_string_lossy();
        let mut arguments = vec![
            "--no-optional-locks",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-C",
            &path,
        ];
        arguments.extend(args);
        let observation = match run_stdout_command(
            &git.executable,
            &arguments,
            neutral,
            Duration::from_secs(5),
        ) {
            Ok(text) => {
                if name == "tracked worktree" {
                    let mut counts = BTreeMap::<String, usize>::new();
                    for line in text.lines() {
                        *counts.entry(line.chars().take(2).collect()).or_default() += 1;
                    }
                    Observation::known(if counts.is_empty() {
                        "Clean (tracked files only)".into()
                    } else {
                        counts
                            .into_iter()
                            .map(|(status, count)| format!("{status}: {count}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                } else {
                    Observation::known(text.trim())
                }
            }
            Err(error) => Observation::unknown(error.to_string()),
        };
        snapshot
            .observations
            .insert(format!("Git / {name}"), observation);
    }
}

fn runtime_constraints(
    project: &Path,
    tools: &[Tool],
    errors: &mut Vec<String>,
) -> Vec<RuntimeConstraint> {
    let mut declarations = Vec::<(String, String, String)>::new();
    for name in ["package.json", "pyproject.toml", "Cargo.toml"] {
        let bytes = match read_file(&project.join(name), 1024 * 1024) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => continue,
            Err(_) => {
                errors.push(format!(
                    "Couldn't read {name} runtime declarations (limit 1 MiB)."
                ));
                continue;
            }
        };
        if name == "package.json" {
            match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(value) => {
                    for tool in ["node", "npm"] {
                        if let Some(declared) = value
                            .get("engines")
                            .and_then(|value| value.get(tool))
                            .and_then(|value| value.as_str())
                        {
                            declarations.push((
                                tool.into(),
                                format!("package.json / engines.{tool}"),
                                declared.into(),
                            ));
                        }
                    }
                }
                Err(_) => {
                    errors.push("package.json is invalid; runtime declarations unknown.".into())
                }
            }
        } else {
            match String::from_utf8(bytes)
                .ok()
                .and_then(|text| text.parse::<toml::Table>().ok())
            {
                Some(value) => {
                    let (tool, table, key) = if name == "pyproject.toml" {
                        ("python", "project", "requires-python")
                    } else {
                        ("rustc", "package", "rust-version")
                    };
                    if let Some(declared) = value
                        .get(table)
                        .and_then(|value| value.get(key))
                        .and_then(|value| value.as_str())
                    {
                        declarations.push((
                            tool.into(),
                            format!("{name} / {table}.{key}"),
                            declared.into(),
                        ));
                    }
                    if name == "Cargo.toml"
                        && value
                            .get("package")
                            .and_then(|value| value.get("rust-version"))
                            .is_some_and(|value| value.is_table())
                    {
                        errors.push("Cargo.toml: inherited rust-version requires workspace resolution; no compatibility claim is made.".into());
                    }
                }
                None => errors.push(format!("{name} is invalid; runtime declarations unknown.")),
            }
        }
    }
    declarations.retain(|(_, source, declared)| {
        if declared.len() <= 512 {
            true
        } else {
            errors.push(format!(
                "{source}: runtime declaration exceeds 512 bytes; unknown."
            ));
            false
        }
    });
    declarations
        .into_iter()
        .map(|(tool, source, declared)| {
            let current = tools
                .iter()
                .find(|entry| entry.id == tool && entry.active)
                .and_then(|entry| entry.version.clone());
            let result = current
                .as_deref()
                .and_then(|version| satisfies(&tool, version, &declared));
            RuntimeConstraint {
                tool,
                source,
                declared,
                current,
                result: match result {
                    Some(true) => "Within declared range",
                    Some(false) => "Outside declared range",
                    None => "Unknown — version or declaration could not be evaluated",
                }
                .into(),
            }
        })
        .collect()
}

fn satisfies(tool: &str, version: &str, declared: &str) -> Option<bool> {
    match tool {
        "node" | "npm" => {
            let version = version.parse::<node_semver::Version>().ok()?;
            let range = declared.parse::<node_semver::Range>().ok()?;
            Some(version.satisfies(&range))
        }
        "python" => {
            let version = version.parse::<pep440_rs::Version>().ok()?;
            // Pre-release policy needs information about the selected Python environment.
            if version.any_prerelease() {
                return None;
            }
            Some(
                declared
                    .parse::<pep440_rs::VersionSpecifiers>()
                    .ok()?
                    .contains(&version),
            )
        }
        "rustc" => {
            let version = version.parse::<node_semver::Version>().ok()?;
            let minimum = if declared.split('.').count() == 2 {
                format!("{declared}.0")
            } else {
                declared.into()
            };
            Some(version >= minimum.parse::<node_semver::Version>().ok()?)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_and_inherited_runtime_declarations_remain_unknown() {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("package.json"),
            serde_json::to_vec(&serde_json::json!({
                "engines": {"node": "x".repeat(513)}
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(
            root.path().join("Cargo.toml"),
            "[package]\nrust-version.workspace = true\n",
        )
        .unwrap();
        let mut errors = Vec::new();
        assert!(runtime_constraints(root.path(), &[], &mut errors).is_empty());
        assert!(
            errors
                .iter()
                .any(|error| error.contains("package.json") && error.contains("512"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("Cargo.toml") && error.contains("inherited"))
        );
    }
    #[test]
    fn ecosystem_ranges_are_not_interchangeable() {
        assert_eq!(satisfies("node", "22.5.1", ">=20 <21"), Some(false));
        assert_eq!(satisfies("node", "20.17.0", "^18 || ^20"), Some(true));
        assert_eq!(satisfies("node", "20.17.0", "lts/*"), None);
        assert_eq!(
            satisfies("python", "3.11.9", ">=3.10,!=3.11.*,<4"),
            Some(false)
        );
        assert_eq!(satisfies("python", "3.12.0rc1", ">=3.10"), None);
        assert_eq!(satisfies("rustc", "1.98.1", "1.85"), Some(true));
    }
    #[test]
    fn hashes_detect_change_without_storing_contents_and_unknown_is_not_unchanged() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("Cargo.lock"), "secret fixture text").unwrap();
        let previous = capture_files(root.path());
        assert!(
            !serde_json::to_string(&previous)
                .unwrap()
                .contains("secret fixture text")
        );
        fs::write(root.path().join("Cargo.lock"), "different").unwrap();
        let mut current = capture_files(root.path());
        current.observations.insert(
            "Lockfile / yarn.lock".into(),
            Observation::unknown("Locked"),
        );
        let changes = compare_snapshots(&previous, &current);
        assert_eq!(
            changes
                .iter()
                .find(|change| change.field.ends_with("Cargo.lock"))
                .unwrap()
                .state,
            "Changed"
        );
        assert_eq!(
            changes
                .iter()
                .find(|change| change.field.ends_with("yarn.lock"))
                .unwrap()
                .state,
            "Unknown"
        );
    }
}
