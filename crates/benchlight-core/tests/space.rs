use benchlight_core::{Benchlight, Classification};
use std::{fs, sync::atomic::AtomicBool};

#[test]
fn storage_is_measured_once_with_nested_and_overlapping_roots() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("source");
    let project = root.join("node");
    fs::create_dir_all(project.join("node_modules/pkg")).unwrap();
    let manifest = r#"{"dependencies":{"fixture":"1.0.0"}}"#;
    fs::write(project.join("package.json"), manifest).unwrap();
    fs::write(project.join("node_modules/pkg/index.js"), [0; 21]).unwrap();
    let app = Benchlight::open(fixture.path().join("data")).unwrap();
    app.add_root(&root).unwrap();
    app.add_root(&project).unwrap();
    let scan = app.scan(&AtomicBool::new(false)).unwrap();
    assert_eq!(scan.logical_bytes, manifest.len() as i64 + 21);
    let candidates = app.candidates(0, 200).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].logical_bytes, 21);
    assert_eq!(candidates[0].classification, Classification::Rebuildable);
    assert_eq!(
        app.projects(0, 200).unwrap()[0].logical_bytes,
        manifest.len() as i64 + 21
    );
    app.set_exclusions(&[project
        .join("node_modules/pkg")
        .to_string_lossy()
        .into_owned()])
        .unwrap();
    assert_eq!(
        app.scan(&AtomicBool::new(false)).unwrap().logical_bytes,
        manifest.len() as i64
    );
    assert!(!app.candidates(0, 200).unwrap()[0].complete);
}
