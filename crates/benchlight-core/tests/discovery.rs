use benchlight_core::{Benchlight, discovery::detect_project};
use std::{fs, path::Path, sync::atomic::AtomicBool};

#[test]
fn checked_in_manifest_fixtures_are_recognized() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    for name in [
        "node-project",
        "python-project",
        "rust-project",
        "dotnet-project",
        "java-project",
        "mixed-project",
    ] {
        assert!(
            detect_project(&root.join(name)).unwrap().is_some(),
            "{name}"
        );
    }
    assert!(
        detect_project(&root.join("unknown-directory"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn missing_root_is_a_partial_failure_and_other_roots_continue() {
    let fixture = tempfile::tempdir().unwrap();
    let missing = fixture.path().join("missing");
    let valid = fixture.path().join("valid");
    fs::create_dir(&missing).unwrap();
    fs::create_dir(&valid).unwrap();
    fs::write(valid.join("go.mod"), "module fixture").unwrap();
    let app = Benchlight::open(fixture.path().join("data")).unwrap();
    app.add_root(&missing).unwrap();
    app.add_root(&valid).unwrap();
    fs::remove_dir(&missing).unwrap();
    let scan = app.scan(&AtomicBool::new(false)).unwrap();
    assert_eq!(scan.projects, 1);
    assert_eq!(scan.errors, 1);
    assert!(app.scan_errors().unwrap()[0].path.contains("missing"));
}

#[cfg(windows)]
#[test]
fn junctions_outside_roots_and_junction_loops_are_not_followed() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("source");
    let outside = fixture.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("Cargo.toml"), "[package]").unwrap();
    for (name, target) in [("outside-link", &outside), ("loop", &root)] {
        let output = std::process::Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(root.join(name))
            .arg(target)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let app = Benchlight::open(fixture.path().join("data")).unwrap();
    app.add_root(&root).unwrap();
    assert!(app.add_root(&root.join("outside-link")).is_err());
    let scan = app.scan(&AtomicBool::new(false)).unwrap();
    assert_eq!(scan.projects, 0);
    assert_eq!(scan.directories, 1);
    assert!(outside.join("Cargo.toml").exists());
}
