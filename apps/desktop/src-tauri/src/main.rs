#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use benchlight_core::CleanupPlan;
use benchlight_core::ToolReport;
use benchlight_core::{AppStatus, Benchlight, Candidate, CategoryTotal, Project, ScanError};
use clap::Parser;
use std::{
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

type App = Mutex<Benchlight>;
#[tauri::command]
async fn export_logs(path: std::path::PathBuf, app: tauri::State<'_, App>) -> Result<(), String> {
    let data = data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        Benchlight::open(data).and_then(|app| app.export_logs(&path))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
#[tauri::command]
fn working_snapshot(
    path: std::path::PathBuf,
    app: tauri::State<'_, App>,
) -> Result<Option<benchlight_core::snapshots::Snapshot>, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .working_snapshot(&path)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn mark_working(
    path: std::path::PathBuf,
    app: tauri::State<'_, App>,
) -> Result<benchlight_core::snapshots::Snapshot, String> {
    let data = data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        Benchlight::open(data).and_then(|app| app.mark_working(&path))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
#[tauri::command]
async fn compare_working(
    path: std::path::PathBuf,
    app: tauri::State<'_, App>,
) -> Result<benchlight_core::snapshots::Comparison, String> {
    let data = data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        Benchlight::open(data).and_then(|app| app.compare_working(&path))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
#[tauri::command]
fn tools(app: tauri::State<'_, App>) -> Result<ToolReport, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .tools()
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn refresh_tools(app: tauri::State<'_, App>) -> Result<ToolReport, String> {
    let data = data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        Benchlight::open(data).and_then(|app| app.refresh_tools())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
#[tauri::command]
async fn tool_projects(
    id: String,
    offset: u32,
    app: tauri::State<'_, App>,
) -> Result<Vec<Project>, String> {
    let data = data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        Benchlight::open(data).and_then(|app| app.tool_projects(&id, offset, 100))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
#[tauri::command]
fn open_tool_location(path: std::path::PathBuf, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .open_tool_location(&path)
        .map_err(|e| e.to_string())
}
fn data_directory(app: &tauri::State<'_, App>) -> Result<std::path::PathBuf, String> {
    Ok(app
        .lock()
        .map_err(|e| e.to_string())?
        .status()
        .map_err(|e| e.to_string())?
        .data_directory)
}
async fn cleanup_work(
    data: std::path::PathBuf,
    work: impl FnOnce(Benchlight) -> Result<CleanupPlan, benchlight_core::Error> + Send + 'static,
) -> Result<CleanupPlan, String> {
    tauri::async_runtime::spawn_blocking(move || Benchlight::open(data).and_then(work))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn create_cleanup_plan(
    paths: Vec<std::path::PathBuf>,
    app: tauri::State<'_, App>,
) -> Result<CleanupPlan, String> {
    cleanup_work(data_directory(&app)?, move |app| {
        app.create_cleanup_plan(&paths)
    })
    .await
}
#[tauri::command]
fn cleanup_plans(app: tauri::State<'_, App>) -> Result<Vec<CleanupPlan>, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .cleanup_plans()
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn validate_cleanup_plan(id: i64, app: tauri::State<'_, App>) -> Result<CleanupPlan, String> {
    cleanup_work(data_directory(&app)?, move |app| {
        app.validate_cleanup_plan(id)
    })
    .await
}
#[tauri::command]
async fn apply_cleanup_plan(
    id: i64,
    confirmation: String,
    app: tauri::State<'_, App>,
) -> Result<CleanupPlan, String> {
    cleanup_work(data_directory(&app)?, move |app| {
        app.apply_cleanup_plan(id, &confirmation)
    })
    .await
}

#[derive(Default, Clone)]
struct ScanState {
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
}

#[derive(serde::Serialize)]
struct ScanStatus {
    running: bool,
    error: Option<String>,
    status: AppStatus,
}

#[tauri::command]
fn start_scan(app: tauri::State<'_, App>, scan: tauri::State<'_, ScanState>) -> Result<(), String> {
    let data = app
        .lock()
        .map_err(|e| e.to_string())?
        .status()
        .map_err(|e| e.to_string())?
        .data_directory;
    if scan.running.swap(true, Ordering::SeqCst) {
        return Err("A scan is already running.".into());
    }
    scan.cancel.store(false, Ordering::SeqCst);
    *scan.error.lock().map_err(|e| e.to_string())? = None;
    let scan = scan.inner().clone();
    std::thread::spawn(move || {
        let result = Benchlight::open(data).and_then(|app| app.scan(&scan.cancel));
        if let Err(error) = result
            && let Ok(mut value) = scan.error.lock()
        {
            *value = Some(error.to_string());
        }
        scan.running.store(false, Ordering::SeqCst);
    });
    Ok(())
}

#[tauri::command]
fn cancel_scan(scan: tauri::State<'_, ScanState>) {
    scan.cancel.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn scan_status(
    app: tauri::State<'_, App>,
    scan: tauri::State<'_, ScanState>,
) -> Result<ScanStatus, String> {
    Ok(ScanStatus {
        running: scan.running.load(Ordering::SeqCst),
        error: scan.error.lock().map_err(|e| e.to_string())?.clone(),
        status: status(app)?,
    })
}

#[tauri::command]
fn projects(
    offset: u32,
    limit: u32,
    sort: Option<String>,
    descending: Option<bool>,
    app: tauri::State<'_, App>,
) -> Result<Vec<Project>, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .projects_sorted(
            offset,
            limit,
            sort.as_deref().unwrap_or("name"),
            descending.unwrap_or(false),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn scan_errors(app: tauri::State<'_, App>) -> Result<Vec<ScanError>, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .scan_errors()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_project_folder(path: String, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .open_project_folder(Path::new(&path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn candidates(
    offset: u32,
    limit: u32,
    sort: Option<String>,
    descending: Option<bool>,
    app: tauri::State<'_, App>,
) -> Result<Vec<Candidate>, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .candidates_sorted(
            offset,
            limit,
            sort.as_deref().unwrap_or("size"),
            descending.unwrap_or(true),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn category_totals(app: tauri::State<'_, App>) -> Result<Vec<CategoryTotal>, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .category_totals()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_include_caches(enabled: bool, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .set_include_caches(enabled)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_exclusions(paths: Vec<String>, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .set_exclusions(&paths)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_storage_folder(path: String, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .open_storage_folder(Path::new(&path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn status(app: tauri::State<'_, App>) -> Result<AppStatus, String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .status()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_root(path: String, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .add_root(Path::new(&path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_root(path: String, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .remove_root(&path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_theme(theme: String, app: tauri::State<'_, App>) -> Result<(), String> {
    app.lock()
        .map_err(|e| e.to_string())?
        .set_theme(&theme)
        .map_err(|e| e.to_string())
}

fn main() {
    #[derive(Parser)]
    struct Options {
        #[arg(long)]
        data_dir: Option<std::path::PathBuf>,
    }
    let options = Options::parse();
    let opened = match options.data_dir {
        Some(path) => Benchlight::open(path),
        None => Benchlight::open_default(),
    };
    let app = match opened {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Couldn't open Benchlight's local database: {error}");
            benchlight_platform_windows::show_error(&format!(
                "Couldn't open Benchlight's local database.\n\n{error}\n\nCheck folder access and free disk space. A database from a newer Benchlight version requires that version. You can start with --data-dir and a different metadata folder."
            ));
            std::process::exit(1);
        }
    };
    tauri::Builder::default()
        .manage(Mutex::new(app))
        .manage(ScanState::default())
        .invoke_handler(tauri::generate_handler![
            export_logs,
            working_snapshot,
            mark_working,
            compare_working,
            tools,
            refresh_tools,
            tool_projects,
            open_tool_location,
            create_cleanup_plan,
            cleanup_plans,
            validate_cleanup_plan,
            apply_cleanup_plan,
            status,
            add_root,
            remove_root,
            set_theme,
            start_scan,
            cancel_scan,
            scan_status,
            projects,
            scan_errors,
            candidates,
            category_totals,
            set_include_caches,
            set_exclusions,
            open_storage_folder,
            open_project_folder
        ])
        .run(tauri::generate_context!())
        .expect("Couldn't start the Benchlight desktop window");
}
