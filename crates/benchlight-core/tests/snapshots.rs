use benchlight_core::Benchlight;
use std::{fs, sync::atomic::AtomicBool};

#[test]
fn working_state_persists_and_comparison_does_not_replace_it() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    let data = temporary.path().join("data");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("package.json"),
        r#"{"engines":{"node":">=20 <21"}}"#,
    )
    .unwrap();
    fs::write(project.join("package-lock.json"), "first lockfile").unwrap();
    let app = Benchlight::open(data.clone()).unwrap();
    app.add_root(&project).unwrap();
    app.scan(&AtomicBool::new(false)).unwrap();
    assert!(app.compare_working(&project).is_err());
    let executable = std::env::current_exe().unwrap();
    assert!(
        app.mark_working_with_health(&project, &executable, &["--invalid-test-option".into()])
            .is_err()
    );
    assert!(app.working_snapshot(&project).unwrap().is_none());
    let baseline = app
        .mark_working_with_health(&project, &executable, &["--list".into()])
        .unwrap();
    assert!(
        baseline.observations["Health / explicit command"]
            .value
            .is_some()
    );
    fs::write(project.join("package-lock.json"), "changed lockfile").unwrap();
    let comparison = app.compare_working(&project).unwrap();
    assert_eq!(
        comparison
            .changes
            .iter()
            .find(|change| change.field == "Health / explicit command")
            .unwrap()
            .state,
        "Unknown"
    );
    assert_eq!(
        comparison
            .changes
            .iter()
            .find(|change| change.field == "Lockfile / package-lock.json")
            .unwrap()
            .state,
        "Changed"
    );
    assert!(
        comparison
            .current
            .observations
            .iter()
            .filter(|(name, _)| name.starts_with("Environment / "))
            .all(|(_, observation)| ["Present", "Missing"]
                .contains(&observation.value.as_deref().unwrap()))
    );
    drop(app);
    let app = Benchlight::open(data).unwrap();
    let saved = app.working_snapshot(&project).unwrap().unwrap();
    assert_eq!(saved.observations, baseline.observations);
    assert!(app.mark_working(temporary.path()).is_err());
}

#[test]
fn older_working_history_requires_exact_confirmation_and_keeps_latest() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("package.json"), "{}").unwrap();
    let data = temporary.path().join("data");
    let app = Benchlight::open(data.clone()).unwrap();
    app.add_root(&project).unwrap();
    app.scan(&AtomicBool::new(false)).unwrap();
    let first = app.mark_working(&project).unwrap();
    fs::write(project.join("package-lock.json"), "second").unwrap();
    let latest = app.mark_working(&project).unwrap();
    assert_eq!(app.snapshot_history_size(&project).unwrap().records, 2);
    assert!(app.remove_older_snapshots(&project, 2, "REMOVE 1").is_err());
    assert_eq!(app.snapshot_history_size(&project).unwrap().records, 2);
    assert!(
        app.remove_older_snapshots(&project, 3, "REMOVE 2 OLDER SNAPSHOTS")
            .is_err()
    );
    app.remove_older_snapshots(&project, 2, "REMOVE 1 OLDER SNAPSHOTS")
        .unwrap();
    assert_eq!(app.snapshot_history_size(&project).unwrap().records, 1);
    assert_ne!(first.observations, latest.observations);
    drop(app);
    let reopened = Benchlight::open(data).unwrap();
    assert_eq!(
        reopened
            .working_snapshot(&project)
            .unwrap()
            .unwrap()
            .observations,
        latest.observations
    );
}
