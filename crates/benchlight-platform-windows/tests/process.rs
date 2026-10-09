#![cfg(windows)]
use benchlight_platform_windows::process::run_command;
use std::time::{Duration, Instant};

#[test]
fn version_process_timeout_and_output_limit_are_enforced() {
    let fixture = tempfile::tempdir().unwrap();
    let executable = std::env::current_exe().unwrap();
    let start = Instant::now();
    let result = run_command(
        &executable,
        &[
            "--ignored",
            "--exact",
            "blocked_child_fixture",
            "--nocapture",
        ],
        fixture.path(),
        Duration::from_millis(150),
    );
    assert!(result.unwrap_err().to_string().contains("timed out"));
    assert!(start.elapsed() < Duration::from_secs(3));
    let result = run_command(
        &executable,
        &[
            "--ignored",
            "--exact",
            "large_output_fixture",
            "--nocapture",
        ],
        fixture.path(),
        Duration::from_secs(3),
    );
    assert!(result.unwrap_err().to_string().contains("16 KiB"));
    let result = run_command(
        &executable,
        &["--ignored", "--exact", "version_fixture", "--nocapture"],
        fixture.path(),
        Duration::from_secs(3),
    )
    .unwrap();
    assert!(result.contains("fixture 1.2.3"));
}
#[test]
#[ignore = "Internal subprocess fixture; run by the bounded-process test"]
fn blocked_child_fixture() {
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}
#[test]
#[ignore = "Internal subprocess fixture; run by the bounded-process test"]
fn large_output_fixture() {
    print!("{}", "x".repeat(100_000));
}
#[test]
#[ignore = "Internal subprocess fixture; run by the bounded-process test"]
fn version_fixture() {
    println!("fixture 1.2.3");
}
