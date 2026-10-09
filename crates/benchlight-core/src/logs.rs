use crate::{Benchlight, Error};
use std::{fs, io::Write, path::Path};

impl Benchlight {
    pub fn export_logs(&self, destination: &Path) -> Result<(), Error> {
        if !destination.is_absolute()
            || destination
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(Error::InvalidRoot(
                "Choose an absolute export path without parent traversal.".into(),
            ));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| Error::InvalidRoot("Choose a file in an existing folder.".into()))?;
        benchlight_platform_windows::ensure_no_reparse_path(parent)?;
        let plans = self.cleanup_plans()?;
        let operations = plans
            .iter()
            .map(|plan| {
                Ok(serde_json::json!({ "plan":plan.id,"events":self.cleanup_log(plan.id)? }))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let report = serde_json::json!({"version":env!("CARGO_PKG_VERSION"),"exported_at":crate::discovery::timestamp(),"scan":self.status()?.scan,"scan_errors":self.scan_errors()?,"cleanup_operations":operations});
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        serde_json::to_writer_pretty(&mut file, &report)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_export_never_overwrites_an_existing_file() {
        let temporary = tempfile::tempdir().unwrap();
        let app = Benchlight::open(temporary.path().join("data")).unwrap();
        let destination = temporary.path().join("report.json");
        app.export_logs(&destination).unwrap();
        let contents = fs::read(&destination).unwrap();
        assert!(app.export_logs(&destination).is_err());
        assert_eq!(contents, fs::read(&destination).unwrap());
        let report: serde_json::Value = serde_json::from_slice(&contents).unwrap();
        assert!(report.get("cleanup_operations").is_some());
        assert!(report.get("environment").is_none());
    }
}
