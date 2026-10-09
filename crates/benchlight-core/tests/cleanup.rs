#![cfg(windows)]
use benchlight_core::Benchlight;
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

fn fixture() -> (tempfile::TempDir, Benchlight, PathBuf, PathBuf) {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    let artifact = project.join("node_modules");
    fs::create_dir_all(&artifact).unwrap();
    fs::write(
        project.join("package.json"),
        r#"{"dependencies":{"example":"1"}}"#,
    )
    .unwrap();
    fs::write(artifact.join("fixture-package"), "test fixture only").unwrap();
    let app = Benchlight::open(temporary.path().join("metadata")).unwrap();
    app.add_root(&project).unwrap();
    app.scan(&AtomicBool::new(false)).unwrap();
    (
        temporary,
        app,
        fs::canonicalize(project).unwrap(),
        fs::canonicalize(artifact).unwrap(),
    )
}
#[test]
fn plan_is_persisted_and_never_mutates_the_target() {
    let (_temporary, app, project, artifact) = fixture();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    assert_eq!(plan.items[0].fingerprint.logical_bytes, 17);
    assert_eq!(app.cleanup_plan(plan.id).unwrap().state, "preview");
    assert!(app.apply_cleanup_plan(plan.id, "yes").is_err());
    assert!(artifact.join("fixture-package").exists());
    assert!(app.create_cleanup_plan(&[project]).is_err());
    assert!(
        app.create_cleanup_plan(&[artifact.clone(), artifact])
            .is_err()
    );
}
#[test]
fn changed_contents_and_project_evidence_block_apply() {
    let (_temporary, app, project, artifact) = fixture();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    fs::write(artifact.join("new-file"), "important local change").unwrap();
    assert!(
        app.apply_cleanup_plan(plan.id, &format!("RECYCLE {}", plan.id))
            .is_err()
    );
    assert!(artifact.join("new-file").exists());
    app.scan(&AtomicBool::new(false)).unwrap();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    fs::write(project.join("package.json"), "{}").unwrap();
    assert!(app.validate_cleanup_plan(plan.id).is_err());
    assert!(artifact.exists());
}
#[test]
fn replacement_with_same_files_and_size_is_rejected() {
    let (_temporary, app, _project, artifact) = fixture();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    fs::rename(&artifact, artifact.with_file_name("old-node-modules")).unwrap();
    fs::create_dir(&artifact).unwrap();
    fs::write(artifact.join("fixture-package"), "test fixture only").unwrap();
    assert!(
        app.apply_cleanup_plan(plan.id, &format!("RECYCLE {}", plan.id))
            .is_err()
    );
    assert!(artifact.exists());
}
#[test]
fn new_junction_inside_target_and_outside_target_replacement_are_rejected() {
    let (temporary, app, _project, artifact) = fixture();
    let outside = temporary.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("keep"), "source").unwrap();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    let link = artifact.join("junction");
    let status = std::process::Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(
        app.apply_cleanup_plan(plan.id, &format!("RECYCLE {}", plan.id))
            .is_err()
    );
    fs::remove_dir(&link).unwrap();
    fs::rename(&artifact, artifact.with_file_name("original-artifact")).unwrap();
    let status = std::process::Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(&artifact)
        .arg(&outside)
        .output()
        .unwrap();
    assert!(status.status.success());
    assert!(app.validate_cleanup_plan(plan.id).is_err());
    assert_eq!(fs::read_to_string(outside.join("keep")).unwrap(), "source");
    fs::remove_dir(&artifact).unwrap();
}
#[test]
fn locked_file_blocks_validation_without_changing_anything() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_temporary, app, _project, artifact) = fixture();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    let _locked = fs::OpenOptions::new()
        .write(true)
        .share_mode(0)
        .open(artifact.join("fixture-package"))
        .unwrap();
    assert!(
        app.apply_cleanup_plan(plan.id, &format!("RECYCLE {}", plan.id))
            .is_err()
    );
    assert!(artifact.exists());
}
#[test]
fn confirmed_temporary_fixture_is_recycled_and_logged() {
    let (_temporary, app, project, artifact) = fixture();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    let result = app
        .apply_cleanup_plan(plan.id, &format!("RECYCLE {}", plan.id))
        .unwrap();
    assert_eq!(result.state, "complete", "{result:?}");
    assert_eq!(result.items[0].state, "recycled");
    assert!(!artifact.exists());
    assert!(project.join("package.json").exists());
    assert_eq!(
        app.cleanup_log(plan.id).unwrap().last().unwrap().state,
        "recycled"
    );
    assert!(
        app.apply_cleanup_plan(plan.id, &format!("RECYCLE {}", plan.id))
            .is_err()
    );
}
#[test]
fn native_guard_prevents_target_and_ancestor_renames() {
    let (_temporary, app, project, artifact) = fixture();
    let plan = app
        .create_cleanup_plan(std::slice::from_ref(&artifact))
        .unwrap();
    let _guard = benchlight_platform_windows::cleanup::CleanupGuard::acquire(
        &artifact,
        plan.items[0].candidate.identity.as_ref().unwrap(),
    )
    .unwrap();
    assert!(fs::rename(&artifact, artifact.with_file_name("swapped")).is_err());
    assert!(fs::rename(&project, project.with_file_name("swapped-project")).is_err());
    assert!(artifact.exists());
}
