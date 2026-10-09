use crate::{Benchlight, Candidate, Classification, Error, discovery::timestamp};
use benchlight_platform_windows::{
    cleanup::{CleanupGuard, identity, supported_volume},
    contains_path, ensure_no_reparse_path, path_key,
};
use benchlight_storage::{
    classify_candidate,
    fingerprint::{TreeStamp, evidence_hash, fingerprint},
    modified,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupItem {
    pub candidate: Candidate,
    pub expected_type: String,
    pub fingerprint: TreeStamp,
    pub evidence_hash: Option<String>,
    pub state: String,
    pub staged_path: Option<PathBuf>,
    pub message: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPlan {
    pub id: i64,
    pub created_at: i64,
    pub state: String,
    pub items: Vec<CleanupItem>,
}

fn reject(message: &str) -> Error {
    Error::InvalidRoot(message.into())
}
fn protect(path: &Path) -> Result<(), Error> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(reject(
            "Cleanup requires an absolute path without parent traversal.",
        ));
    }
    let key = path_key(path);
    if key.len() <= 3
        || key[2..].contains(':')
        || path.components().any(|part| {
            part.as_os_str()
                .to_string_lossy()
                .eq_ignore_ascii_case(".git")
        })
    {
        return Err(reject(
            "Filesystem roots, alternate streams and Git metadata are protected.",
        ));
    }
    let (directories, profile) = benchlight_platform_windows::protected_locations()?;
    if directories
        .iter()
        .any(|directory| contains_path(directory, path))
        || path_key(&profile) == key
    {
        return Err(reject(
            "Windows, application installation directories and the user profile root are protected.",
        ));
    }
    for variable in [
        "SystemRoot",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "ProgramData",
    ] {
        if let Some(protected) = std::env::var_os(variable)
            && contains_path(Path::new(&protected), path)
        {
            return Err(reject(
                "Windows and application installation directories are protected.",
            ));
        }
    }
    if let Some(profile) = std::env::var_os("USERPROFILE")
        && path_key(Path::new(&profile)) == key
    {
        return Err(reject("The user profile root is protected."));
    }
    supported_volume(path)?;
    ensure_no_reparse_path(path)?;
    if path_key(&fs::canonicalize(path)?) != key {
        return Err(reject("Cleanup path resolves to a different location."));
    }
    Ok(())
}

impl Benchlight {
    fn cleanup_context(&self, candidate: &Candidate) -> Result<Option<String>, Error> {
        protect(&candidate.path)?;
        if !candidate.complete
            || !matches!(
                candidate.classification,
                Classification::Rebuildable | Classification::Cache
            )
        {
            return Err(reject(
                "Only complete measurements of rebuildable or cache directories can enter an ordinary cleanup plan.",
            ));
        }
        if !candidate.path.is_dir()
            || candidate.identity.as_ref() != Some(&identity(&candidate.path)?)
        {
            return Err(reject(
                "The scanned directory was replaced or lacks filesystem identity. Scan again.",
            ));
        }
        if contains_path(&candidate.path, &self.data_directory)
            || contains_path(&self.data_directory, &candidate.path)
        {
            return Err(reject("Benchlight metadata is protected."));
        }
        for root in self.database.roots()? {
            if path_key(Path::new(&root)) == path_key(&candidate.path) {
                return Err(reject("Selected project roots are protected."));
            }
        }
        let exclusions = self.status()?.exclusions;
        if exclusions.iter().any(|path| {
            contains_path(Path::new(path), &candidate.path)
                || contains_path(&candidate.path, Path::new(path))
        }) {
            return Err(reject("Cleanup overlaps a scan exclusion."));
        }
        if let Some(project) = &candidate.project {
            ensure_no_reparse_path(project)?;
            if candidate.path.parent().map(path_key) != Some(path_key(project)) {
                return Err(reject(
                    "Artifact ownership must be established in its immediate project directory.",
                ));
            }
            let detected = crate::discovery::detect_project(project)?
                .ok_or_else(|| reject("Project evidence is no longer present."))?;
            let current = classify_candidate(&candidate.path, project, &detected.evidence)
                .ok_or_else(|| reject("Artifact classification is no longer supported."))?;
            if current.classification != candidate.classification
                || current.category != candidate.category
            {
                return Err(reject(
                    "Artifact classification changed. Scan and plan again.",
                ));
            }
            Ok(Some(evidence_hash(project, &detected.evidence)?))
        } else {
            if !benchlight_platform_windows::cache_locations()
                .iter()
                .any(|(path, category)| {
                    path_key(path) == path_key(&candidate.path)
                        && *category == candidate.category
                        && !["Maven cache", "Poetry cache"].contains(category)
                })
            {
                return Err(reject(
                    "The path is not an established package-manager cache.",
                ));
            }
            Ok(None)
        }
    }
    pub fn create_cleanup_plan(&self, paths: &[PathBuf]) -> Result<CleanupPlan, Error> {
        if paths.is_empty() || paths.len() > 50 {
            return Err(reject("Select between 1 and 50 directories for a plan."));
        }
        let mut items: Vec<CleanupItem> = Vec::new();
        for path in paths {
            protect(path)?;
            if items.iter().any(|item| {
                contains_path(&item.candidate.path, path)
                    || contains_path(path, &item.candidate.path)
            }) {
                return Err(reject(
                    "Cleanup targets must not duplicate or overlap one another.",
                ));
            }
            let candidate = self.candidate(path)?;
            let evidence = self.cleanup_context(&candidate)?;
            let fingerprint = fingerprint(&candidate.path)?;
            if fingerprint.logical_bytes != candidate.logical_bytes
                || fingerprint.files != candidate.files
                || modified(&candidate.path) != candidate.modified
            {
                return Err(reject(
                    "The directory changed since the scan. Scan again before planning.",
                ));
            }
            items.push(CleanupItem {
                candidate,
                expected_type: "directory".into(),
                fingerprint,
                evidence_hash: evidence,
                state: "pending".into(),
                staged_path: None,
                message: None,
            });
        }
        let mut plan = CleanupPlan {
            id: 0,
            created_at: timestamp(),
            state: "preview".into(),
            items,
        };
        plan.id = self
            .database
            .create_cleanup_plan(plan.created_at, &serde_json::to_string(&plan)?)?;
        self.save_cleanup_plan(&plan)?;
        Ok(plan)
    }
    fn save_cleanup_plan(&self, plan: &CleanupPlan) -> Result<(), Error> {
        self.database
            .update_cleanup_plan(plan.id, &plan.state, &serde_json::to_string(plan)?)?;
        Ok(())
    }
    pub fn cleanup_plan(&self, id: i64) -> Result<CleanupPlan, Error> {
        Ok(serde_json::from_str(
            &self
                .database
                .cleanup_plan(id)?
                .ok_or_else(|| reject("Cleanup plan not found."))?,
        )?)
    }
    pub fn cleanup_plans(&self) -> Result<Vec<CleanupPlan>, Error> {
        self.database
            .cleanup_plans()?
            .into_iter()
            .map(|json| Ok(serde_json::from_str(&json)?))
            .collect()
    }
    pub fn cleanup_log(&self, id: i64) -> Result<Vec<benchlight_db::OperationEvent>, Error> {
        Ok(self.database.operation_log(id)?)
    }
    fn validate_cleanup_item(&self, item: &CleanupItem) -> Result<(), Error> {
        if item.expected_type != "directory"
            || self.cleanup_context(&item.candidate)? != item.evidence_hash
            || fingerprint(&item.candidate.path)? != item.fingerprint
        {
            return Err(reject(
                "Filesystem or project evidence changed after review. Create a new plan.",
            ));
        }
        Ok(())
    }
    pub fn validate_cleanup_plan(&self, id: i64) -> Result<CleanupPlan, Error> {
        let plan = self.cleanup_plan(id)?;
        if plan.state != "preview" {
            return Err(reject(
                "This plan has already been attempted; inspect its result log.",
            ));
        }
        for item in &plan.items {
            self.validate_cleanup_item(item)?;
        }
        Ok(plan)
    }
    pub fn apply_cleanup_plan(&self, id: i64, confirmation: &str) -> Result<CleanupPlan, Error> {
        if confirmation != format!("RECYCLE {id}") {
            return Err(reject(
                "Review the plan and explicitly confirm with RECYCLE followed by its plan number.",
            ));
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.data_directory.join("cleanup.lock"))?;
        lock.try_lock()
            .map_err(|_| reject("Another cleanup is running."))?;
        let mut plan = self.validate_cleanup_plan(id)?;
        plan.state = "applying".into();
        self.save_cleanup_plan(&plan)?;
        for index in 0..plan.items.len() {
            let result = (|| -> Result<(), Error> {
                let item = &plan.items[index];
                let expected = item
                    .candidate
                    .identity
                    .as_ref()
                    .ok_or_else(|| reject("No target identity."))?;
                let mut guard = CleanupGuard::acquire(&item.candidate.path, expected)?;
                let _evidence = item
                    .candidate
                    .project
                    .as_ref()
                    .map(|project| {
                        let mut names = item.candidate.evidence.clone();
                        names.extend(
                            [
                                "package-lock.json",
                                "pnpm-lock.yaml",
                                "yarn.lock",
                                "Cargo.lock",
                            ]
                            .map(str::to_owned),
                        );
                        names.sort();
                        names.dedup();
                        benchlight_platform_windows::cleanup::lock_evidence(project, &names)
                    })
                    .transpose()?;
                self.validate_cleanup_item(item)?;
                let staged = guard.staging_path()?;
                let original = item.candidate.path.clone();
                plan.items[index].staged_path = Some(staged.clone());
                plan.items[index].state = "staging".into();
                self.save_cleanup_plan(&plan)?;
                self.database.log_operation(
                    id,
                    timestamp(),
                    &original.to_string_lossy(),
                    "staging",
                    &staged.to_string_lossy(),
                )?;
                guard.stage(&staged)?;
                plan.items[index].state = "staged".into();
                self.save_cleanup_plan(&plan)?;
                if fingerprint(&staged)? != plan.items[index].fingerprint {
                    return Err(reject(
                        "Staged contents changed; retained at the recovery path. Nothing was recycled.",
                    ));
                }
                guard.recycle(
                    plan.items[index]
                        .candidate
                        .identity
                        .as_ref()
                        .ok_or_else(|| reject("No target identity."))?,
                )?;
                Ok(())
            })();
            let (state, message) = match result {
                Ok(()) => (
                    "recycled",
                    "Moved to the Windows Recycle Bin. Restore the staged name there; the log records its original path.".to_owned(),
                ),
                Err(error) => ("failed", error.to_string()),
            };
            if state == "failed"
                && plan.items[index]
                    .staged_path
                    .as_ref()
                    .is_some_and(|path| !path.exists())
            {
                plan.items[index].staged_path = None;
            }
            plan.items[index].state = state.into();
            plan.items[index].message = Some(message.clone());
            self.save_cleanup_plan(&plan)?;
            self.database.log_operation(
                id,
                timestamp(),
                &plan.items[index].candidate.path.to_string_lossy(),
                state,
                &message,
            )?;
            if state == "failed" {
                break;
            }
        }
        plan.state = if plan.items.iter().all(|item| item.state == "recycled") {
            "complete"
        } else {
            "partial"
        }
        .into();
        self.save_cleanup_plan(&plan)?;
        Ok(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dangerous_paths_are_rejected_before_candidate_lookup() {
        for path in [
            PathBuf::from(r"C:\"),
            PathBuf::from(r"C:\Windows"),
            PathBuf::from(r"C:\Program Files"),
            PathBuf::from(r"C:\source\..\Windows"),
            PathBuf::from(r"\\server\share"),
            PathBuf::from(r"C:\source\.git"),
        ] {
            assert!(protect(&path).is_err(), "{}", path.display());
        }
        if let Some(profile) = std::env::var_os("USERPROFILE") {
            assert!(protect(Path::new(&profile)).is_err());
        }
    }
}
