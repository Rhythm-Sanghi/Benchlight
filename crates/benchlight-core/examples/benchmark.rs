use benchlight_core::Benchlight;
use std::{
    fs,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("projects");
    for project in 0..50 {
        let path = root.join(format!("project-{project}"));
        fs::create_dir_all(path.join("node_modules/package"))?;
        fs::write(path.join("package.json"), "{}")?;
        for file in 0..400 {
            fs::write(
                path.join("node_modules/package").join(format!("{file}.js")),
                [0; 512],
            )?;
        }
    }
    let app = Benchlight::open(fixture.path().join("metadata"))?;
    app.add_root(&root)?;
    let start = Instant::now();
    let scan = app.scan(&AtomicBool::new(false))?;
    let duration = start.elapsed();
    let query_start = Instant::now();
    for _ in 0..100 {
        std::hint::black_box(app.candidates(0, 100)?);
    }
    let query_duration = query_start.elapsed();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let directory = fixture.path().join("metadata");
    let worker = std::thread::spawn(move || {
        Benchlight::open(directory)
            .unwrap()
            .scan(&worker_cancel)
            .unwrap()
    });
    std::thread::sleep(Duration::from_millis(100));
    let requested = Instant::now();
    cancel.store(true, Ordering::SeqCst);
    let cancelled = worker.join().expect("benchmark worker failed");
    println!(
        "fixture_files=20050 fixture_projects=50 logical_bytes={} scan_ms={} query_100_pages_ms={} cancellation_ms={} cancellation_state={}",
        scan.logical_bytes,
        duration.as_millis(),
        query_duration.as_millis(),
        requested.elapsed().as_millis(),
        cancelled.state
    );
    Ok(())
}
