use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use anyhow::Result;

pub const STATE_FILE_NAME: &str = ".tagger_state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistedState {
    pub selected_dataset: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub legacy_positions: std::collections::HashMap<String, usize>,
}

pub fn load_state(path: &Path) -> PersistedState {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str(&data) {
            return parsed;
        }
    }
    PersistedState::default()
}

pub fn save_state_file(path: &Path, state: &PersistedState) -> Result<()> {
    let json = serde_json::to_string_pretty(state)?;
    fs::write(path, json)?;
    Ok(())
}
