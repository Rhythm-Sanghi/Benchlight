#![cfg(windows)]
use benchlight_core::Benchlight;
use std::{fs, sync::atomic::AtomicBool};
#[test]
fn long_hidden_directory_paths_are_measured_without_truncation() {
    let fixture = tempfile::tempdir().unwrap();
    let project = fixture.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("package.json"),
        r#"{"dependencies":{"fixture":"1"}}"#,
    )
    .unwrap();
    let artifact = project.join("node_modules");
    let mut leaf = artifact.clone();
    for index in 0..15 {
        leaf = leaf.join(format!(".hidden-long-directory-{index}"));
    }
    fs::create_dir_all(&leaf).unwrap();
    fs::write(leaf.join("fixture"), [0; 37]).unwrap();
    assert!(leaf.to_string_lossy().len() > 260);
    let app = Benchlight::open(fixture.path().join("data")).unwrap();
    app.add_root(&project).unwrap();
    let scan = app.scan(&AtomicBool::new(false)).unwrap();
    assert_eq!(scan.errors, 0);
    let candidates = app.candidates(0, 100).unwrap();
    assert_eq!(candidates[0].logical_bytes, 37);
    assert!(candidates[0].complete);
    let plan = app
        .create_cleanup_plan(&[candidates[0].path.clone()])
        .unwrap();
    assert_eq!(plan.items[0].fingerprint.logical_bytes, 37);
}
