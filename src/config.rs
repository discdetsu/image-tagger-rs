use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub dataset_root: PathBuf,
    pub finding_name: String,
    pub state_file: PathBuf,
}

impl AppConfig {
    pub fn discover() -> Result<Self> {
        if let Ok(manual) = env::var("DATASET_ROOT") {
            let manual_path = PathBuf::from(manual);
            return Self::from_root(manual_path);
        }

        if let Some(arg_root) = env::args().skip(1).next() {
            if !arg_root.trim().is_empty() {
                return Self::from_root(PathBuf::from(arg_root));
            }
        }

        // 1. Check current working directory
        let current_dir = env::current_dir().context("Read current directory")?;
        if is_dataset_dir(&current_dir)? {
            return Self::from_root(current_dir);
        }

        // 2. Check executable directory (often the case when double-clicking)
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                if is_dataset_dir(exe_dir)? {
                    return Self::from_root(exe_dir.to_path_buf());
                }
            }
        }

        // 3. Walk up from current directory
        let mut current = current_dir.clone();
        for _ in 0..6 {
            if is_dataset_dir(&current)? {
                return Self::from_root(current);
            }

            if !current.pop() {
                break;
            }
        }

        // Fallback: prefer executable directory, then current directory
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                return Self::from_root(exe_dir.to_path_buf());
            }
        }

        Self::from_root(current_dir)
    }

    pub fn from_root(root: PathBuf) -> Result<Self> {
        let canonical = root.canonicalize().unwrap_or(root);

        let finding_name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Finding")
            .to_string();

        let state_file = canonical.join(crate::state::STATE_FILE_NAME);

        Ok(Self {
            dataset_root: canonical,
            finding_name,
            state_file,
        })
    }
}

pub fn is_dataset_dir(dir: &Path) -> Result<bool> {
    if !dir.is_dir() {
        return Ok(false);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("csv") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if dir.join(stem).is_dir() {
                    return Ok(true);
                }
            }
        }
    }

    Ok(false)
}
