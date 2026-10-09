use std::process::Command;

#[test]
fn cli_persists_roots_and_reports_json_errors_with_exit_codes() {
    let temporary = tempfile::tempdir().unwrap();
    let data = temporary.path().join("data");
    let root = temporary.path().join("projects");
    std::fs::create_dir(&root).unwrap();
    let binary = env!("CARGO_BIN_EXE_benchlight");
    let output = Command::new(binary)
        .arg("--data-dir")
        .arg(&data)
        .args(["roots", "add"])
        .arg(&root)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let roots: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(roots.len(), 1);
    let output = Command::new(binary)
        .arg("--data-dir")
        .arg(&data)
        .args(["status", "--quiet"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    let output = Command::new(binary)
        .arg("--data-dir")
        .arg(&data)
        .args(["roots", "add", "..\\outside"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("absolute"));
    let output = Command::new(binary)
        .arg("unknown-command")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}
