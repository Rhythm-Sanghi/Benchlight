use std::{
    env,
    fs::Metadata,
    io,
    path::{Path, PathBuf},
    process::Command,
};
#[cfg(windows)]
pub mod cleanup;
pub mod installations;
pub mod process;

fn known_folder(id: &windows::core::GUID) -> io::Result<PathBuf> {
    use windows::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath},
    };
    let value =
        unsafe { SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None) }.map_err(io::Error::other)?;
    let path = unsafe { value.to_string() }
        .map(PathBuf::from)
        .map_err(io::Error::other);
    unsafe { CoTaskMemFree(Some(value.0.cast())) };
    path
}

pub fn protected_locations() -> io::Result<(Vec<PathBuf>, PathBuf)> {
    use windows::Win32::UI::Shell::*;
    let directories = [
        FOLDERID_Windows,
        FOLDERID_ProgramFiles,
        FOLDERID_ProgramFilesX86,
        FOLDERID_ProgramData,
    ]
    .iter()
    .map(known_folder)
    .collect::<io::Result<Vec<_>>>()?;
    Ok((directories, known_folder(&FOLDERID_Profile)?))
}

pub fn ensure_local_root(path: &Path) -> io::Result<()> {
    let key = path_key(path);
    if key.starts_with(r"\\") {
        return Err(io::Error::other(
            "Network roots are not supported. Choose a local project folder.",
        ));
    }
    if key.as_bytes().get(1) == Some(&b':') {
        use std::os::windows::ffi::OsStrExt;
        let root: Vec<u16> = std::ffi::OsStr::new(&format!("{}\\", &key[..2]))
            .encode_wide()
            .chain(Some(0))
            .collect();
        if unsafe {
            windows::Win32::Storage::FileSystem::GetDriveTypeW(windows::core::PCWSTR(root.as_ptr()))
        } == 4
        {
            return Err(io::Error::other(
                "Mapped network roots are not supported. Choose a local project folder.",
            ));
        }
    }
    Ok(())
}

pub fn open_folder(path: &Path) -> io::Result<()> {
    let windows = known_folder(&windows::Win32::UI::Shell::FOLDERID_Windows)?;
    Command::new(windows.join("explorer.exe"))
        .arg(path)
        .spawn()?;
    Ok(())
}

pub fn show_error(message: &str) {
    use std::os::windows::ffi::OsStrExt;
    use windows::{Win32::UI::WindowsAndMessaging::*, core::PCWSTR};
    let text: Vec<u16> = std::ffi::OsStr::new(message)
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            windows::core::w!("Benchlight"),
            MB_OK | MB_ICONERROR,
        )
    };
}

pub fn cache_locations() -> Vec<(PathBuf, &'static str)> {
    let Some(profile) = env::var_os("USERPROFILE").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut paths = vec![
        (profile.join(".cargo/registry/cache"), "Cargo package cache"),
        (profile.join(".cargo/git/db"), "Cargo Git cache"),
        (profile.join(".gradle/caches"), "Gradle cache"),
        (profile.join(".m2/repository"), "Maven cache"),
        (profile.join(".nuget/packages"), "NuGet cache"),
        (profile.join(".bun/install/cache"), "Bun cache"),
    ];
    if let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        paths.extend([
            (local.join("npm-cache"), "npm cache"),
            (local.join("pnpm/store"), "pnpm store"),
            (local.join("Yarn/Cache"), "Yarn cache"),
            (local.join("pip/Cache"), "pip cache"),
            (local.join("uv/cache"), "uv cache"),
            (local.join("pypoetry/Cache"), "Poetry cache"),
        ]);
    }
    paths
}

pub struct ManagedStorage {
    pub locations: Vec<(PathBuf, &'static str)>,
    pub errors: Vec<(PathBuf, io::Error)>,
}

pub fn managed_storage_locations() -> ManagedStorage {
    let mut locations = Vec::new();
    let mut errors = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        locations.push((local.join("Docker/wsl"), "Docker Desktop data"));
    }
    if let Some(profile) = env::var_os("USERPROFILE").map(PathBuf::from) {
        locations.push((profile.join(".android/avd"), "Android emulators"));
    }
    #[cfg(windows)]
    {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        let key_path = r"Software\Microsoft\Windows\CurrentVersion\Lxss";
        match RegKey::predef(HKEY_CURRENT_USER).open_subkey(key_path) {
            Ok(key) => {
                for name in key.enum_keys().take(1024) {
                    let inspected = name.and_then(|name| {
                        let entry = key.open_subkey(name)?;
                        let base: String = entry.get_value("BasePath")?;
                        Ok(PathBuf::from(base).join("ext4.vhdx"))
                    });
                    match inspected {
                        Ok(path) => locations.push((path, "WSL virtual disks")),
                        Err(error) => errors.push((PathBuf::from(key_path), error)),
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => errors.push((PathBuf::from(key_path), error)),
        }
    }
    ManagedStorage { locations, errors }
}

pub fn data_directory() -> io::Result<PathBuf> {
    Ok(known_folder(&windows::Win32::UI::Shell::FOLDERID_LocalAppData)?.join("Benchlight"))
}

pub fn path_key(path: &Path) -> String {
    let path = path.to_string_lossy().replace('/', "\\");
    let path = if let Some(unc) = path.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{unc}")
    } else {
        path.strip_prefix("\\\\?\\").unwrap_or(&path).to_owned()
    };
    path.trim_end_matches('\\').to_lowercase()
}

pub fn contains_path(parent: &Path, path: &Path) -> bool {
    let parent = path_key(parent);
    let path = path_key(path);
    path == parent || path.starts_with(&(parent + "\\"))
}

pub fn ensure_no_reparse_path(path: &Path) -> io::Result<()> {
    let ancestors: Vec<_> = path.ancestors().collect();
    for ancestor in ancestors.into_iter().rev() {
        if is_reparse_point(&std::fs::symlink_metadata(ancestor)?) {
            return Err(io::Error::other(
                "Path contains a link, junction or cloud placeholder; skipped.",
            ));
        }
    }
    Ok(())
}

pub fn suggested_roots() -> Vec<PathBuf> {
    let Some(profile) = env::var_os("USERPROFILE") else {
        return Vec::new();
    };
    let profile = PathBuf::from(profile);
    [
        profile.join("source"),
        profile.join("Documents").join("Projects"),
    ]
    .into_iter()
    .filter(|path| path.is_dir())
    .filter_map(|path| std::fs::canonicalize(path).ok())
    .collect()
}

pub fn is_reparse_point(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Reparse points include junctions and cloud placeholders; scanning them
        // could leave the chosen root or trigger a download.
        attributes_need_skipping(metadata.file_attributes())
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn attributes_need_skipping(attributes: u32) -> bool {
    // Offline/recall flags may occur without a reparse bit. Reading their data
    // can hydrate a cloud file, so metadata inspection skips them too.
    attributes & (0x400 | 0x1000 | 0x40000 | 0x400000) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_and_nonresident_cloud_attributes_are_skipped() {
        for attribute in [0x400, 0x1000, 0x40000, 0x400000] {
            assert!(attributes_need_skipping(attribute));
        }
        assert!(!attributes_need_skipping(0x80));
        assert!(!attributes_need_skipping(0x10 | 0x2));
    }
}
