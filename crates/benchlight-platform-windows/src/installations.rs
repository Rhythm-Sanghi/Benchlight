use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApplication {
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub location: Option<String>,
    pub registry_key: String,
    pub winget_id: Option<String>,
}
pub fn installed_applications() -> (Vec<InstalledApplication>, Vec<String>) {
    use winreg::{RegKey, enums::*};
    let mut applications = Vec::new();
    let mut errors = Vec::new();
    for (hive, label) in [(HKEY_CURRENT_USER, "HKCU"), (HKEY_LOCAL_MACHINE, "HKLM")] {
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            let key_path = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
            match RegKey::predef(hive).open_subkey_with_flags(key_path, KEY_READ | view) {
                Ok(key) => {
                    for name in key.enum_keys().take(4096) {
                        let inspected = name.and_then(|name| {
                            let item = key.open_subkey(&name)?;
                            let display: String = item.get_value("DisplayName")?;
                            Ok(InstalledApplication {
                                name: display,
                                version: item.get_value("DisplayVersion").ok(),
                                publisher: item.get_value("Publisher").ok(),
                                location: item.get_value("InstallLocation").ok(),
                                registry_key: format!("{label}\\{key_path}\\{name}"),
                                winget_id: item.get_value("WinGetPackageIdentifier").ok(),
                            })
                        });
                        match inspected {
                            Ok(item) => {
                                if !applications.iter().any(|old: &InstalledApplication| {
                                    old.registry_key == item.registry_key
                                }) {
                                    applications.push(item);
                                }
                            }
                            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                            Err(error) => {
                                errors.push(format!("{label} uninstall metadata: {error}"))
                            }
                        }
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
                Err(error) => errors.push(format!("{label} uninstall registry: {error}")),
            }
        }
    }
    (applications, errors)
}
