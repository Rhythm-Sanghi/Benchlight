use crate::{
    Benchlight, Candidate, Error, Project, ScanRun,
    discovery::{detect_project_with_cancel, timestamp},
};
use benchlight_db::ProjectRecord;
use benchlight_platform_windows::is_reparse_point;
use benchlight_storage::{classify_candidate, measure_directory_excluding, modified};
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

struct Directory {
    entries: fs::ReadDir,
    path: PathBuf,
    project: Option<Project>,
    owner: Option<PathBuf>,
    evidence: Vec<String>,
    bytes: i64,
}

fn bytes(value: u64) -> i64 {
    value.try_into().unwrap_or(i64::MAX)
}

fn within(path: &Path, parent: &Path) -> bool {
    benchlight_platform_windows::contains_path(parent, path)
}

impl Benchlight {
    fn scan_error(
        &self,
        scan: &mut ScanRun,
        path: &Path,
        failure: &io::Error,
    ) -> Result<(), Error> {
        scan.errors += 1;
        if scan.errors <= 500 {
            self.database
                .add_scan_error(scan.id, &path.to_string_lossy(), &failure.to_string())?;
        }
        Ok(())
    }

    fn measure_candidate(
        &self,
        scan: &mut ScanRun,
        candidate: &mut Candidate,
        cancel: &AtomicBool,
    ) -> Result<i64, Error> {
        let mut failures = Vec::new();
        let original_identity =
            benchlight_platform_windows::cleanup::identity(&candidate.path).ok();
        let before = scan.clone();
        let mut tick = Instant::now();
        let mut progress_error = None;
        let mut exclusions: Vec<PathBuf> = self
            .status()?
            .exclusions
            .into_iter()
            .map(PathBuf::from)
            .collect();
        exclusions.push(self.data_directory.clone());
        let measurement = measure_directory_excluding(
            &candidate.path,
            cancel,
            &exclusions,
            |path, error| {
                if failures.len() < 500 {
                    failures.push((path.to_path_buf(), error.to_string()));
                }
            },
            |measurement| {
                if tick.elapsed().as_millis() >= 200 {
                    let mut progress = before.clone();
                    progress.logical_bytes = progress
                        .logical_bytes
                        .saturating_add(bytes(measurement.logical_bytes));
                    progress.errors += bytes(measurement.errors);
                    progress.skipped += bytes(measurement.skipped);
                    if let Err(error) = self.database.update_scan(&progress) {
                        progress_error = Some(error);
                    }
                    tick = Instant::now();
                }
            },
        );
        if let Some(error) = progress_error {
            return Err(error.into());
        }
        for (path, error) in failures
            .into_iter()
            .take((500 - scan.errors.min(500)) as usize)
        {
            self.database
                .add_scan_error(scan.id, &path.to_string_lossy(), &error)?;
        }
        scan.errors += bytes(measurement.errors);
        scan.skipped += bytes(measurement.skipped);
        scan.logical_bytes = scan
            .logical_bytes
            .saturating_add(bytes(measurement.logical_bytes));
        candidate.logical_bytes = measurement.logical_bytes;
        candidate.files = measurement.files;
        candidate.modified = modified(&candidate.path);
        candidate.complete =
            measurement.errors == 0 && measurement.skipped == 0 && !measurement.cancelled;
        candidate.identity = benchlight_platform_windows::cleanup::identity(&candidate.path).ok();
        candidate.complete &=
            candidate.identity.is_some() && candidate.identity == original_identity;
        self.database.save_candidate(
            scan.id,
            &candidate.path.to_string_lossy(),
            &candidate.category,
            &format!("{:?}", candidate.classification),
            bytes(candidate.logical_bytes),
            &serde_json::to_string(candidate)?,
        )?;
        Ok(bytes(measurement.logical_bytes))
    }

    pub fn scan(&self, cancel: &AtomicBool) -> Result<ScanRun, Error> {
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.data_directory.join("scan.lock"))?;
        lock.try_lock().map_err(|_| {
            Error::InvalidRoot(
                "Another Benchlight scan is running. Cancel it or wait for it to finish.".into(),
            )
        })?;
        let mut roots: Vec<PathBuf> = self
            .database
            .roots()?
            .into_iter()
            .map(PathBuf::from)
            .collect();
        roots.sort_by_key(|path| path.components().count());
        let mut selected: Vec<PathBuf> = Vec::new();
        for root in roots {
            if !selected.iter().any(|parent| within(&root, parent)) {
                selected.push(root);
            }
        }
        if selected.is_empty() {
            return Err(Error::InvalidRoot(
                "Add a project folder before scanning.".into(),
            ));
        }
        let mut scan = ScanRun {
            id: self.database.begin_scan(timestamp())?,
            started_at: timestamp(),
            finished_at: None,
            state: "running".into(),
            directories: 0,
            projects: 0,
            errors: 0,
            logical_bytes: 0,
            skipped: 0,
        };
        let mut tick = Instant::now();
        for root in &selected {
            if let Err(error) = benchlight_platform_windows::ensure_no_reparse_path(root) {
                self.scan_error(&mut scan, root, &error)?;
                continue;
            }
            let exclusions: Vec<String> = self
                .database
                .setting("exclusions")?
                .map(|json| serde_json::from_str(&json))
                .transpose()?
                .unwrap_or_default();
            let mut frames: Vec<Directory> = Vec::new();
            let mut pending = Some(root.clone());
            loop {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                let path = if let Some(path) = pending.take() {
                    path
                } else {
                    let Some(frame) = frames.last_mut() else {
                        break;
                    };
                    match frame.entries.next() {
                        Some(Ok(entry)) => entry.path(),
                        Some(Err(error)) => {
                            let path = frame.path.clone();
                            self.scan_error(&mut scan, &path, &error)?;
                            continue;
                        }
                        None => {
                            let frame = frames.pop().expect("frame exists");
                            if let Some(project) = frame.project {
                                self.database.set_project_size(
                                    scan.id,
                                    &project.path.to_string_lossy(),
                                    frame.bytes,
                                )?;
                            }
                            if let Some(parent) = frames.last_mut() {
                                parent.bytes = parent.bytes.saturating_add(frame.bytes);
                            }
                            continue;
                        }
                    }
                };
                if within(&path, &self.data_directory) {
                    scan.skipped += 1;
                    continue;
                }
                if exclusions
                    .iter()
                    .any(|excluded| within(&path, Path::new(excluded)))
                {
                    scan.skipped += 1;
                    continue;
                }
                let metadata = match fs::symlink_metadata(&path) {
                    Ok(value) => value,
                    Err(error) => {
                        self.scan_error(&mut scan, &path, &error)?;
                        continue;
                    }
                };
                if is_reparse_point(&metadata) {
                    scan.skipped += 1;
                    continue;
                }
                if metadata.is_file() {
                    let size = bytes(metadata.len());
                    scan.logical_bytes = scan.logical_bytes.saturating_add(size);
                    if let Some(frame) = frames.last_mut() {
                        frame.bytes = frame.bytes.saturating_add(size);
                    }
                } else if metadata.is_dir() {
                    if frames.len() >= 128 {
                        self.scan_error(
                            &mut scan,
                            &path,
                            &io::Error::other("Directory depth limit reached."),
                        )?;
                        continue;
                    }
                    scan.directories += 1;
                    let candidate = frames.last().and_then(|frame| {
                        frame
                            .owner
                            .as_ref()
                            .and_then(|owner| classify_candidate(&path, owner, &frame.evidence))
                    });
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_ascii_lowercase();
                    if let Some(mut candidate) = candidate {
                        let measured = self.measure_candidate(&mut scan, &mut candidate, cancel)?;
                        if let Some(frame) = frames.last_mut() {
                            frame.bytes = frame.bytes.saturating_add(measured);
                        }
                    } else if [".git", "node_modules", ".venv", "venv", "__pycache__"]
                        .contains(&name.as_str())
                    {
                        let mut candidate = Candidate {
                            path: path.clone(),
                            project: None,
                            category: "Unclassified environments".into(),
                            classification: crate::Classification::Protected,
                            reason:
                                "Ownership cannot be established from surrounding project evidence."
                                    .into(),
                            evidence: vec![],
                            recreate: None,
                            logical_bytes: 0,
                            files: 0,
                            complete: false,
                            modified: None,
                            identity: None,
                        };
                        let size = self.measure_candidate(&mut scan, &mut candidate, cancel)?;
                        if let Some(frame) = frames.last_mut() {
                            frame.bytes = frame.bytes.saturating_add(size);
                        }
                    } else {
                        let project = match detect_project_with_cancel(&path, cancel) {
                            Ok(project) => project,
                            Err(error) => {
                                self.scan_error(&mut scan, &path, &error)?;
                                None
                            }
                        };
                        if let Some(project) = &project {
                            let row = ProjectRecord {
                                path: project.path.to_string_lossy().into_owned(),
                                name: project.name.clone(),
                                evidence: serde_json::to_string(&project.evidence)?,
                                languages: serde_json::to_string(&project.languages)?,
                                last_activity: project.last_activity,
                                logical_bytes: 0,
                            };
                            if self.database.save_project(scan.id, &row)? {
                                scan.projects += 1;
                            }
                        }
                        let (owner, evidence) = match &project {
                            Some(project) => (Some(project.path.clone()), project.evidence.clone()),
                            None => frames
                                .last()
                                .map(|frame| (frame.owner.clone(), frame.evidence.clone()))
                                .unwrap_or_default(),
                        };
                        match fs::read_dir(&path) {
                            Ok(entries) => frames.push(Directory {
                                entries,
                                path: path.clone(),
                                project,
                                owner,
                                evidence,
                                bytes: 0,
                            }),
                            Err(error) => self.scan_error(&mut scan, &path, &error)?,
                        }
                    }
                }
                if tick.elapsed().as_millis() >= 200 {
                    self.database.update_scan(&scan)?;
                    tick = Instant::now();
                }
            }
        }
        if self.database.setting("include_caches")?.as_deref() == Some("true") {
            for (path, category) in benchlight_platform_windows::cache_locations() {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                if !path.is_dir()
                    || selected
                        .iter()
                        .any(|root| within(&path, root) || within(root, &path))
                {
                    continue;
                }
                let mut candidate = Candidate { path, project: None, category: category.into(), classification: if category == "Maven cache" || category == "Poetry cache" { crate::Classification::Review } else { crate::Classification::Cache },
                    reason: "Known per-user package-manager cache location; packages may need downloading again.".into(),
                    evidence: vec!["Standard package-manager cache location".into()], recreate: None, logical_bytes: 0, files: 0, complete: false, modified: None, identity: None };
                self.measure_candidate(&mut scan, &mut candidate, cancel)?;
            }
            let managed = benchlight_platform_windows::managed_storage_locations();
            for (path, error) in managed.errors {
                self.scan_error(&mut scan, &path, &error)?;
            }
            let mut measured_managed: Vec<PathBuf> = Vec::new();
            let mut locations = managed.locations;
            locations.sort_by_key(|(path, _)| path.components().count());
            for (path, category) in locations {
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                match fs::symlink_metadata(&path) {
                    Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                    Err(error) => {
                        self.scan_error(&mut scan, &path, &error)?;
                        continue;
                    }
                    Ok(_) => (),
                }
                if selected
                    .iter()
                    .any(|root| within(&path, root) || within(root, &path))
                    || measured_managed.iter().any(|root| within(&path, root))
                {
                    continue;
                }
                measured_managed.push(path.clone());
                let mut candidate = Candidate { path, project: None, category: category.into(),
                    classification: if category == "Android emulators" { crate::Classification::Review } else { crate::Classification::Protected },
                    reason: if category == "Android emulators" { "Emulator images can contain user data and need individual review." } else { "Virtual disks can contain distributions and persistent volumes. Do not delete them. File length is not engine usage or reclaimable space." }.into(),
                    evidence: vec![if category == "WSL virtual disks" { "WSL per-user distribution registry metadata" } else { "Standard per-user application data location" }.into()],
                    recreate: None, logical_bytes: 0, files: 0, complete: false, modified: None, identity: None };
                self.measure_candidate(&mut scan, &mut candidate, cancel)?;
            }
        }
        scan.state = if cancel.load(Ordering::Relaxed) {
            "cancelled"
        } else {
            "complete"
        }
        .into();
        scan.finished_at = Some(timestamp());
        self.database.update_scan(&scan)?;
        Ok(scan)
    }
}
