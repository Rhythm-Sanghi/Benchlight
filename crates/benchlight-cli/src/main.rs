use benchlight_core::Benchlight;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "A local system utility for developers")]
struct Arguments {
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    quiet: bool,
    #[arg(long, global = true)]
    no_color: bool,
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    ExportLogs {
        destination: PathBuf,
    },
    MarkGood {
        project: PathBuf,
        #[arg(long)]
        health_executable: Option<PathBuf>,
        #[arg(long, requires = "health_executable", allow_hyphen_values = true)]
        health_arg: Vec<String>,
    },
    Changes {
        project: PathBuf,
    },
    WorkingState {
        project: PathBuf,
    },
    Tools {
        #[arg(long)]
        refresh: bool,
    },
    Inspect {
        tool_or_path: String,
    },
    ToolProjects {
        tool: String,
        #[arg(long, default_value_t = 0)]
        offset: u32,
    },
    Cleanup {
        #[command(subcommand)]
        command: Cleanup,
    },
    Status {
        project: Option<PathBuf>,
    },
    Scan,
    ScanErrors,
    Space {
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    CacheScan {
        enabled: bool,
    },
    Exclusions {
        paths: Vec<String>,
    },
    Projects {
        #[arg(long, default_value_t = 0)]
        offset: u32,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    Roots {
        #[command(subcommand)]
        command: Roots,
    },
}

#[derive(Subcommand)]
enum Roots {
    List,
    Add { path: PathBuf },
    Remove { path: String },
}
#[derive(Subcommand)]
enum Cleanup {
    Plan {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
    },
    List,
    Show {
        plan: i64,
    },
    Validate {
        plan: i64,
    },
    Log {
        plan: i64,
    },
    Apply {
        plan: i64,
        #[arg(long)]
        confirm: String,
    },
}

fn run(arguments: Arguments) -> Result<u8, Box<dyn std::error::Error>> {
    let app = match arguments.data_dir {
        Some(path) => Benchlight::open(path)?,
        None => Benchlight::open_default()?,
    };
    let mut exit = 0;
    match arguments.command {
        Command::ExportLogs { destination } => app.export_logs(&destination)?,
        Command::MarkGood {
            project,
            health_executable,
            health_arg,
        } => {
            let snapshot = match health_executable {
                Some(executable) => {
                    app.mark_working_with_health(&project, &executable, &health_arg)?
                }
                None => app.mark_working(&project)?,
            };
            if !arguments.quiet {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            }
        }
        Command::Changes { project } => {
            let comparison = app.compare_working(&project)?;
            if !arguments.quiet {
                println!("{}", serde_json::to_string_pretty(&comparison)?);
            }
        }
        Command::WorkingState { project } => {
            let snapshot = app.working_snapshot(&project)?;
            if !arguments.quiet {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            }
        }
        Command::Tools { refresh } => {
            let report = if refresh {
                app.refresh_tools()?
            } else {
                app.tools()?
            };
            if !arguments.quiet {
                if arguments.json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    for tool in report.tools {
                        println!(
                            "{}\t{}\t{}\t{}",
                            tool.name,
                            tool.version.as_deref().unwrap_or("Unknown"),
                            tool.executable.display(),
                            if tool.active {
                                "First PATH match"
                            } else {
                                "Shadowed"
                            }
                        );
                    }
                    for error in report.errors {
                        eprintln!("benchlight: {error}");
                    }
                }
            }
        }
        Command::Inspect { tool_or_path } => {
            let tool = app.inspect_tool(&tool_or_path)?;
            if !arguments.quiet {
                println!("{}", serde_json::to_string_pretty(&tool)?);
            }
        }
        Command::ToolProjects { tool, offset } => {
            let projects = app.tool_projects(&tool, offset, 100)?;
            if !arguments.quiet {
                println!("{}", serde_json::to_string_pretty(&projects)?);
            }
        }
        Command::Cleanup { command } => {
            let value = match command {
                Cleanup::Plan { paths } => serde_json::to_value(app.create_cleanup_plan(&paths)?)?,
                Cleanup::List => serde_json::to_value(app.cleanup_plans()?)?,
                Cleanup::Show { plan } => serde_json::to_value(app.cleanup_plan(plan)?)?,
                Cleanup::Validate { plan } => {
                    serde_json::to_value(app.validate_cleanup_plan(plan)?)?
                }
                Cleanup::Log { plan } => serde_json::to_value(app.cleanup_log(plan)?)?,
                Cleanup::Apply { plan, confirm } => {
                    let result = app.apply_cleanup_plan(plan, &confirm)?;
                    if result.state != "complete" {
                        exit = 3;
                    }
                    serde_json::to_value(result)?
                }
            };
            if !arguments.quiet {
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        }
        Command::Space { offset, limit } => {
            let totals = app.category_totals()?;
            let candidates = app.candidates(offset, limit)?;
            if !arguments.quiet {
                if arguments.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &serde_json::json!({ "scan": app.status()?.scan, "categories": totals, "candidates": candidates })
                        )?
                    );
                } else {
                    for item in totals {
                        println!(
                            "{}\t{} bytes\t{} dirs\t{}",
                            item.category, item.logical_bytes, item.items, item.classification
                        );
                    }
                }
            }
        }
        Command::CacheScan { enabled } => app.set_include_caches(enabled)?,
        Command::Exclusions { paths } => app.set_exclusions(&paths)?,
        Command::ScanErrors => {
            let errors = app.scan_errors()?;
            if !arguments.quiet {
                if arguments.json {
                    println!("{}", serde_json::to_string_pretty(&errors)?);
                } else {
                    for error in errors {
                        println!("{}\t{}", error.path, error.message);
                    }
                }
            }
        }
        Command::Scan => {
            let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let handler = cancel.clone();
            ctrlc::set_handler(move || handler.store(true, std::sync::atomic::Ordering::Relaxed))?;
            let scan = app.scan(&cancel)?;
            if scan.errors > 0 {
                exit = 3;
            }
            if scan.state == "cancelled" {
                exit = 130;
            }
            if !arguments.quiet {
                if arguments.json {
                    println!("{}", serde_json::to_string_pretty(&scan)?);
                } else {
                    println!(
                        "Scan {}. {} projects; {} directories; {} errors.",
                        scan.state, scan.projects, scan.directories, scan.errors
                    );
                }
            }
        }
        Command::Projects { offset, limit } => {
            let projects = app.projects(offset, limit)?;
            if !arguments.quiet {
                if arguments.json {
                    println!("{}", serde_json::to_string_pretty(&projects)?);
                } else {
                    for project in projects {
                        println!(
                            "{}\t{}\t{}",
                            project.name,
                            project.languages.join(" · "),
                            project.path.display()
                        );
                    }
                }
            }
        }
        Command::Status { project } => {
            if let Some(project) = project {
                let snapshot = app.working_snapshot(&project)?;
                if !arguments.quiet {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &serde_json::json!({"project":project,"last_working":snapshot})
                        )?
                    );
                }
                return Ok(exit);
            }
            let status = app.status()?;
            if !arguments.quiet {
                if arguments.json {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                } else {
                    println!(
                        "Benchlight {}\nLocal data: {}\nProject roots: {}",
                        status.version,
                        status.data_directory.display(),
                        status.roots.len()
                    );
                }
            }
        }
        Command::Roots { command } => {
            match command {
                Roots::Add { path } => app.add_root(&path)?,
                Roots::Remove { path } => app.remove_root(&path)?,
                Roots::List => (),
            }
            if !arguments.quiet {
                let roots = app.status()?.roots;
                if arguments.json {
                    println!("{}", serde_json::to_string_pretty(&roots)?);
                } else {
                    for path in roots {
                        println!("{path}");
                    }
                }
            }
        }
    }
    Ok(exit)
}

fn main() -> std::process::ExitCode {
    match run(Arguments::parse()) {
        Ok(exit) => std::process::ExitCode::from(exit),
        Err(error) => {
            eprintln!("benchlight: {error}");
            std::process::ExitCode::from(1)
        }
    }
}
