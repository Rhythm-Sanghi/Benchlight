use benchlight_platform_windows::{cleanup::identity, is_reparse_point};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io, path::Path, time::UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeStamp {
    pub logical_bytes: u64,
    pub files: u64,
    pub entries: u64,
    pub digest: String,
}

pub fn fingerprint(path: &Path) -> io::Result<TreeStamp> {
    benchlight_platform_windows::ensure_no_reparse_path(path)?;
    let mut stamp = TreeStamp {
        logical_bytes: 0,
        files: 0,
        entries: 0,
        digest: String::new(),
    };
    let mut hash = Sha256::new();
    visit(path, path, 0, &mut hash, &mut stamp)?;
    stamp.digest = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(stamp)
}
fn visit(
    root: &Path,
    path: &Path,
    depth: usize,
    hash: &mut Sha256,
    stamp: &mut TreeStamp,
) -> io::Result<()> {
    if depth > 128 || stamp.entries >= 1_000_000 {
        return Err(io::Error::other(
            "Cleanup validation limit reached; no files were changed.",
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if is_reparse_point(&metadata) {
        return Err(io::Error::other(
            "Cleanup does not support links or cloud placeholders, including inside a target.",
        ));
    }
    if !metadata.is_dir() && !metadata.is_file() {
        return Err(io::Error::other("Unsupported file type."));
    }
    let relative = path
        .strip_prefix(root)
        .map_err(io::Error::other)?
        .to_string_lossy();
    let id = identity(path)?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    hash.update(
        serde_json::to_vec(&(
            relative.as_ref(),
            metadata.is_dir(),
            metadata.len(),
            modified,
            id,
        ))
        .map_err(io::Error::other)?,
    );
    stamp.entries += 1;
    if metadata.is_file() {
        benchlight_platform_windows::cleanup::ensure_idle_file(path)?;
        stamp.files += 1;
        stamp.logical_bytes = stamp
            .logical_bytes
            .checked_add(metadata.len())
            .ok_or_else(|| io::Error::other("Size overflow."))?;
    } else {
        let mut children = Vec::new();
        for child in fs::read_dir(path)? {
            if children.len() >= 100_000 {
                return Err(io::Error::other(
                    "Cleanup validation directory limit reached.",
                ));
            }
            children.push(child?.path());
        }
        children.sort();
        for child in children {
            visit(root, &child, depth + 1, hash, stamp)?;
        }
    }
    Ok(())
}

pub fn evidence_hash(project: &Path, evidence: &[String]) -> io::Result<String> {
    use std::io::Read;
    let mut hash = Sha256::new();
    let mut names = evidence.to_vec();
    names.extend(
        [
            "package-lock.json",
            "pnpm-lock.yaml",
            "yarn.lock",
            "Cargo.lock",
        ]
        .map(str::to_owned),
    );
    names.sort();
    names.dedup();
    for name in names {
        if name.contains(['/', '\\']) || name == ".." {
            return Err(io::Error::other("Invalid project evidence."));
        }
        let path = project.join(&name);
        if !path.exists() {
            hash.update(name.as_bytes());
            hash.update(b"missing");
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        if is_reparse_point(&metadata) {
            return Err(io::Error::other("Project evidence now contains a link."));
        }
        if metadata.is_file() {
            let mut bytes = Vec::new();
            fs::File::open(&path)?
                .take(1_048_577)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 1_048_576 {
                return Err(io::Error::other(
                    "Project evidence exceeds the validation limit.",
                ));
            }
            hash.update(name.as_bytes());
            hash.update(Sha256::digest(&bytes));
        }
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
