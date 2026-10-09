use crate::{Benchlight, Error, Project, discovery::timestamp};
use benchlight_platform_windows::{
    contains_path,
    installations::{InstalledApplication, installed_applications},
    path_key,
    process::run_command,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

struct ToolSpec {
    id: &'static str,
    name: &'static str,
    args: &'static [&'static str],
    manifests: &'static [&'static str],
}
const CATALOG: &[ToolSpec] = &[
    ToolSpec {
        id: "git",
        name: "Git",
        args: &["--version"],
        manifests: &[".git"],
    },
    ToolSpec {
        id: "node",
        name: "Node.js",
        args: &["--version"],
        manifests: &["package.json"],
    },
    ToolSpec {
        id: "npm",
        name: "npm",
        args: &["--version"],
        manifests: &["package.json"],
    },
    ToolSpec {
        id: "pnpm",
        name: "pnpm",
        args: &["--version"],
        manifests: &["package.json"],
    },
    ToolSpec {
        id: "yarn",
        name: "Yarn",
        args: &["--version"],
        manifests: &["package.json"],
    },
    ToolSpec {
        id: "bun",
        name: "Bun",
        args: &["--version"],
        manifests: &["package.json"],
    },
    ToolSpec {
        id: "python",
        name: "Python",
        args: &["--version"],
        manifests: &["pyproject.toml", "requirements.txt"],
    },
    ToolSpec {
        id: "pip",
        name: "pip",
        args: &["--version"],
        manifests: &["pyproject.toml", "requirements.txt"],
    },
    ToolSpec {
        id: "uv",
        name: "uv",
        args: &["--version"],
        manifests: &["pyproject.toml", "requirements.txt"],
    },
    ToolSpec {
        id: "java",
        name: "Java",
        args: &["-version"],
        manifests: &["pom.xml", "build.gradle", "build.gradle.kts"],
    },
    ToolSpec {
        id: "gradle",
        name: "Gradle",
        args: &["--version"],
        manifests: &["build.gradle", "build.gradle.kts"],
    },
    ToolSpec {
        id: "mvn",
        name: "Maven",
        args: &["--version"],
        manifests: &["pom.xml"],
    },
    ToolSpec {
        id: "go",
        name: "Go",
        args: &["version"],
        manifests: &["go.mod"],
    },
    ToolSpec {
        id: "rustc",
        name: "Rust",
        args: &["--version"],
        manifests: &["Cargo.toml"],
    },
    ToolSpec {
        id: "cargo",
        name: "Cargo",
        args: &["--version"],
        manifests: &["Cargo.toml"],
    },
    ToolSpec {
        id: "dotnet",
        name: ".NET",
        args: &["--version"],
        manifests: &["*.csproj", "*.sln"],
    },
    ToolSpec {
        id: "docker",
        name: "Docker",
        args: &["--version"],
        manifests: &["compose.yaml", "docker-compose.yml", "Dockerfile"],
    },
    ToolSpec {
        id: "wsl",
        name: "WSL",
        args: &["--version"],
        manifests: &[],
    },
    ToolSpec {
        id: "cmake",
        name: "CMake",
        args: &["--version"],
        manifests: &["CMakeLists.txt"],
    },
    ToolSpec {
        id: "ninja",
        name: "Ninja",
        args: &["--version"],
        manifests: &["build.ninja"],
    },
    ToolSpec {
        id: "gh",
        name: "GitHub CLI",
        args: &["--version"],
        manifests: &[".git"],
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    pub likely_source: Option<String>,
    pub confidence: String,
    pub evidence: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub executable: PathBuf,
    pub active: bool,
    pub path_entry: Option<String>,
    pub path_index: Option<usize>,
    pub version: Option<String>,
    pub version_source: Option<String>,
    pub error: Option<String>,
    pub provenance: Provenance,
    pub observed_at: i64,
    pub related_projects: u64,
}
#[derive(Debug, Serialize)]
pub struct ToolReport {
    pub tools: Vec<Tool>,
    pub errors: Vec<String>,
}

pub fn parse_version(output: &str) -> Option<String> {
    // Runtime versions must include a dotted numeric component; vendor labels are not guessed.
    output.split_whitespace().find_map(|word| {
        let value = word.trim_matches(|ch: char| {
            ch == 'v' || ch == '"' || ch == '\'' || ch == '(' || ch == ')' || ch == ',' || ch == ';'
        });
        if value.len() <= 80
            && value.chars().next().is_some_and(|ch| ch.is_ascii_digit())
            && value.contains('.')
            && value
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ".-+_".contains(ch))
        {
            Some(value.to_owned())
        } else {
            None
        }
    })
}
fn bounded_text(path: &Path, limit: u64) -> Option<String> {
    let mut contents = String::new();
    fs::File::open(path)
        .ok()?
        .take(limit + 1)
        .read_to_string(&mut contents)
        .ok()?;
    (contents.len() as u64 <= limit).then_some(contents)
}
fn package_version(path: &Path, id: &str) -> Option<String> {
    let parent = path.parent()?;
    for file in [
        parent.join("node_modules").join(id).join("package.json"),
        parent.join(format!("node_modules/{id}/bin/../package.json")),
    ] {
        let value: serde_json::Value =
            serde_json::from_str(&bounded_text(&file, 1_048_576)?).ok()?;
        if value.get("name")?.as_str()? == id {
            return value.get("version")?.as_str().and_then(parse_version);
        }
    }
    None
}
fn distribution_version(path: &Path, id: &str) -> Option<String> {
    let prefix = match id {
        "gradle" => "gradle-launcher-",
        "mvn" => "maven-core-",
        _ => return None,
    };
    let library = path.parent()?.parent()?.join("lib");
    let matches: Vec<String> = fs::read_dir(library)
        .ok()?
        .take(2000)
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name = name.to_str()?;
            let version = name.strip_prefix(prefix)?.strip_suffix(".jar")?;
            parse_version(version)
        })
        .collect();
    (matches.len() == 1).then(|| matches[0].clone())
}
fn provenance(path: &Path, name: &str, applications: &[InstalledApplication]) -> Provenance {
    let mut result = Provenance {
        likely_source: None,
        confidence: "Unknown".into(),
        evidence: vec![],
    };
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.into());
    let key = path_key(path);
    if let Ok(metadata) = fs::metadata(path) {
        result
            .evidence
            .push(format!("Executable size: {} bytes.", metadata.len()));
    }
    for root in [
        std::env::var_os("LOCALAPPDATA")
            .map(|value| PathBuf::from(value).join("Microsoft/WinGet/Packages")),
        std::env::var_os("ProgramFiles").map(|value| PathBuf::from(value).join("WinGet/Packages")),
    ]
    .into_iter()
    .flatten()
    {
        if contains_path(&root, &canonical) {
            result.likely_source = Some("winget".into());
            result.confidence = "Medium".into();
            result.evidence.push(format!(
                "Executable under WinGet portable package storage: {}",
                root.display()
            ));
        }
    }
    for app in applications {
        let location = app
            .location
            .as_ref()
            .filter(|location| !location.trim().is_empty())
            .map(PathBuf::from);
        if location
            .as_ref()
            .is_some_and(|location| location.is_absolute() && contains_path(location, &canonical))
        {
            result.evidence.push(format!(
                "Windows uninstall entry: {}{}; {}",
                app.name,
                app.version
                    .as_ref()
                    .map(|v| format!(" {v}"))
                    .unwrap_or_default(),
                app.registry_key
            ));
            if let Some(id) = &app.winget_id {
                result.likely_source = Some("winget".into());
                result.confidence = "High".into();
                result
                    .evidence
                    .push(format!("Recorded WinGetPackageIdentifier: {id}"));
            }
        } else if app.name.eq_ignore_ascii_case(name) {
            result.evidence.push(format!(
                "Name-matching Windows uninstall entry: {} (executable location not established)",
                app.registry_key
            ));
            if result.confidence == "Unknown" {
                result.confidence = "Low".into();
            }
        }
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        let scoop = PathBuf::from(profile).join("scoop");
        if contains_path(&scoop, &canonical) || contains_path(&scoop, path) {
            let shim = path.with_extension("shim");
            if shim.is_file() || key.contains("\\scoop\\apps\\") {
                result.likely_source = Some("Scoop".into());
                result.confidence = "Medium".into();
                result.evidence.push(format!(
                    "Executable or shim under Scoop storage: {}",
                    path.display()
                ));
            }
        }
    }
    let chocolatey = std::env::var_os("ChocolateyInstall")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("ProgramData").map(|p| PathBuf::from(p).join("chocolatey")));
    if let Some(root) = chocolatey
        && (contains_path(&root, path) || contains_path(&root, &canonical))
    {
        result.likely_source = Some("Chocolatey".into());
        result.confidence = "Medium".into();
        result.evidence.push(format!(
            "Executable under Chocolatey install storage: {}",
            root.display()
        ));
    }
    if result.likely_source.is_none() {
        result
            .evidence
            .push("Installation channel could not be established from local metadata.".into());
    }
    result
}
fn relates(spec: &ToolSpec, project: &Project) -> bool {
    spec.manifests.iter().any(|marker| {
        project.evidence.iter().any(|evidence| {
            if let Some(suffix) = marker.strip_prefix('*') {
                evidence.to_lowercase().ends_with(suffix)
            } else {
                evidence.eq_ignore_ascii_case(marker)
            }
        }) || (!marker.starts_with('*') && project.path.join(marker).is_file())
    })
}

impl Benchlight {
    pub fn refresh_tools(&self) -> Result<ToolReport, Error> {
        let (applications, mut errors) = installed_applications();
        let mut tools = Vec::new();
        let directories: Vec<PathBuf> =
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .take(512)
                .collect();
        for (index, directory) in directories.iter().enumerate() {
            if !directory.is_absolute() {
                errors.push(format!(
                    "Ignored relative/empty PATH entry at position {index}."
                ));
            }
        }
        for spec in CATALOG {
            if tools.len() >= 200 {
                break;
            }
            let mut active = true;
            let related_count = self.tool_project_count(spec.id)?;
            for (index, directory) in directories
                .iter()
                .enumerate()
                .filter(|(_, path)| path.is_absolute())
            {
                let path = ["exe", "com", "cmd", "bat"]
                    .iter()
                    .map(|extension| directory.join(format!("{}.{extension}", spec.id)))
                    .find(|path| path.is_file());
                let Some(path) = path else { continue };
                if tools
                    .iter()
                    .any(|tool: &Tool| path_key(&tool.executable) == path_key(&path))
                {
                    continue;
                }
                let (version, source, error) = if path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                    && !path_key(&path).contains("\\microsoft\\windowsapps\\")
                {
                    match run_command(&path,spec.args,&self.data_directory,Duration::from_secs(5)) {
                        Ok(output)=>match parse_version(&output) {Some(version)=>(Some(version),Some("Version command".into()),None),None=>(None,None,Some("No dotted numeric version could be established from command output.".into()))},
                        Err(error)=>(None,None,Some(error.to_string()))
                    }
                } else if let Some(version) = package_version(&path, spec.id) {
                    (
                        Some(version),
                        Some("Adjacent package metadata (wrapper not executed)".into()),
                        None,
                    )
                } else if let Some(version) = distribution_version(&path, spec.id) {
                    (
                        Some(version),
                        Some(
                            "Adjacent distribution library filename (wrapper not executed)".into(),
                        ),
                        None,
                    )
                } else {
                    (None,None,Some("Script wrapper or application alias detected; it was not executed. No matching package version metadata found.".into()))
                };
                tools.push(Tool {
                    id: spec.id.into(),
                    name: spec.name.into(),
                    executable: path.clone(),
                    active,
                    path_entry: Some(directory.to_string_lossy().into_owned()),
                    path_index: Some(index),
                    version,
                    version_source: source,
                    error,
                    provenance: provenance(&path, spec.name, &applications),
                    observed_at: timestamp(),
                    related_projects: related_count,
                });
                active = false;
                if tools.len() >= 200 {
                    errors.push("Tool observation limit (200) reached.".into());
                    break;
                }
            }
        }
        let records = tools
            .iter()
            .map(|tool| {
                Ok((
                    tool.executable.to_string_lossy().into_owned(),
                    tool.name.clone(),
                    serde_json::to_string(tool)?,
                ))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        self.database
            .save_tools(&records, &serde_json::to_string(&errors)?)?;
        Ok(ToolReport { tools, errors })
    }
    pub fn tools(&self) -> Result<ToolReport, Error> {
        Ok(ToolReport {
            tools: self
                .database
                .tools()?
                .into_iter()
                .map(|json| Ok(serde_json::from_str(&json)?))
                .collect::<Result<_, Error>>()?,
            errors: self
                .database
                .setting("tool_errors")?
                .map(|json| serde_json::from_str(&json))
                .transpose()?
                .unwrap_or_default(),
        })
    }
    pub fn inspect_tool(&self, input: &str) -> Result<Tool, Error> {
        let report = self.tools()?;
        if let Some(tool) = report.tools.iter().find(|tool| {
            tool.id.eq_ignore_ascii_case(input) && tool.active
                || path_key(&tool.executable) == path_key(Path::new(input))
        }) {
            return Ok(tool.clone());
        }
        let path = Path::new(input);
        if !path.is_absolute() || !path.is_file() {
            return Err(Error::InvalidRoot(
                "Refresh tools first, or supply an absolute executable path.".into(),
            ));
        }
        let (applications, _) = installed_applications();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Ok(Tool {
            id: name.clone(),
            name: name.clone(),
            executable: path.into(),
            active: false,
            path_entry: None,
            path_index: None,
            version: None,
            version_source: None,
            error: Some(
                "Inspected installation metadata only; arbitrary executables are not run.".into(),
            ),
            provenance: provenance(path, &name, &applications),
            observed_at: timestamp(),
            related_projects: 0,
        })
    }
    pub fn open_tool_location(&self, path: &Path) -> Result<(), Error> {
        if !self
            .tools()?
            .tools
            .iter()
            .any(|tool| path_key(&tool.executable) == path_key(path))
        {
            return Err(Error::InvalidRoot(
                "The executable is not in your tool observations.".into(),
            ));
        }
        let parent = path
            .parent()
            .ok_or_else(|| Error::InvalidRoot("Executable has no parent location.".into()))?;
        benchlight_platform_windows::open_folder(&fs::canonicalize(parent)?)?;
        Ok(())
    }
    pub fn tool_projects(&self, id: &str, offset: u32, limit: u32) -> Result<Vec<Project>, Error> {
        let Some(spec) = CATALOG.iter().find(|spec| spec.id == id) else {
            return Ok(vec![]);
        };
        let mut result = Vec::new();
        let mut page = 0;
        let mut matched = 0;
        loop {
            let projects = self.projects(page, 200)?;
            let count = projects.len();
            for project in projects {
                if relates(spec, &project) {
                    if matched >= offset && result.len() < (limit.min(200) as usize) {
                        result.push(project);
                    }
                    matched += 1;
                }
            }
            if count < 200 || result.len() >= limit.min(200) as usize {
                break;
            }
            page += 200;
        }
        Ok(result)
    }
    fn tool_project_count(&self, id: &str) -> Result<u64, Error> {
        let Some(spec) = CATALOG.iter().find(|spec| spec.id == id) else {
            return Ok(0);
        };
        let mut offset = 0;
        let mut total = 0;
        loop {
            let projects = self.projects(offset, 200)?;
            let count = projects.len();
            total += projects
                .iter()
                .filter(|project| relates(spec, project))
                .count() as u64;
            if count < 200 {
                break;
            }
            offset += 200;
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions_are_numeric_and_do_not_guess_vendor_labels() {
        assert_eq!(parse_version("node v22.5.1"), Some("22.5.1".into()));
        assert_eq!(parse_version("rustc 1.98.1 (hash)"), Some("1.98.1".into()));
        assert_eq!(
            parse_version("Docker version 29.5.0, build example"),
            Some("29.5.0".into())
        );
        assert_eq!(
            parse_version("openjdk version \"21.0.2\""),
            Some("21.0.2".into())
        );
        assert!(parse_version("unknown development tool").is_none());
    }
    #[test]
    fn a_matching_registry_name_alone_never_proves_an_installation_channel() {
        let path = Path::new(r"C:\unrelated\node.exe");
        let app = InstalledApplication {
            name: "Node.js".into(),
            version: Some("22.0.0".into()),
            publisher: None,
            location: None,
            registry_key: "fixture uninstall".into(),
            winget_id: Some("OpenJS.NodeJS".into()),
        };
        let source = provenance(path, "Node.js", &[app]);
        assert_eq!(source.confidence, "Low");
        assert!(source.likely_source.is_none());
    }
    #[test]
    fn related_project_lists_are_bounded_and_counts_include_later_pages() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("projects");
        fs::create_dir(&root).unwrap();
        for number in 0..205 {
            let project = root.join(format!("project-{number:03}"));
            fs::create_dir(&project).unwrap();
            fs::write(project.join("package.json"), "{}").unwrap();
        }
        let app = Benchlight::open(fixture.path().join("data")).unwrap();
        app.add_root(&root).unwrap();
        app.scan(&std::sync::atomic::AtomicBool::new(false))
            .unwrap();
        assert_eq!(app.tool_project_count("node").unwrap(), 205);
        assert_eq!(app.tool_projects("node", 200, 100).unwrap().len(), 5);
        assert!(app.tool_projects("rustc", 0, 100).unwrap().is_empty());
    }
}
