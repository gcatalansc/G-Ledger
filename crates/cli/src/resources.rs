use crate::errors::CliError;
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

include!(concat!(env!("OUT_DIR"), "/resource_index.rs"));

pub fn catalog() -> serde_json::Value {
    serde_json::json!({"profile":"project-ledger-resources/1", "files": FILES.iter().map(|(path, size, sha256)| {
        serde_json::json!({"path":path,"size":size,"sha256":sha256})
    }).collect::<Vec<_>>()})
}

fn invalid() -> CliError {
    CliError::new(
        15,
        "RESOURCE_INTEGRITY",
        "matching installation resources are missing or modified; use --resources with this binary's resource set",
    )
}

pub fn resolve(explicit: Option<&Path>) -> Result<PathBuf, CliError> {
    let root = if let Some(path) = explicit {
        path.to_path_buf()
    } else {
        let executable = env::current_exe().map_err(|_| invalid())?;
        let directory = executable.parent().ok_or_else(invalid)?;
        let installed = directory.join("resources");
        if fs::symlink_metadata(&installed).is_ok() {
            installed
        } else if matches!(
            directory.file_name().and_then(|s| s.to_str()),
            Some("debug" | "release")
        ) && directory
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|n| n == "target")
        {
            // Development fallback is anchored to the executable, never the working directory.
            directory
                .parent()
                .and_then(Path::parent)
                .ok_or_else(invalid)?
                .to_path_buf()
        } else {
            return Err(invalid());
        }
    };
    verify(&root)?;
    Ok(root)
}

fn verify(root: &Path) -> Result<(), CliError> {
    for (relative, size, expected) in FILES {
        let mut path = root.to_path_buf();
        for component in Path::new(relative).components() {
            path.push(component);
            if fs::symlink_metadata(&path)
                .map_err(|_| invalid())?
                .file_type()
                .is_symlink()
            {
                return Err(invalid());
            }
        }
        let metadata = fs::symlink_metadata(&path).map_err(|_| invalid())?;
        if !metadata.is_file() || metadata.len() != *size {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|_| invalid())?
            .take(size + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| invalid())?;
        if u64::try_from(bytes.len()).ok() != Some(*size)
            || &project_ledger_canonical_json::sha256_bytes(&bytes).to_string()[7..] != *expected
        {
            return Err(invalid());
        }
    }
    Ok(())
}
