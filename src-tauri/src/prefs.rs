//! User preferences, persisted as JSON in the app config directory.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prefs {
    pub notifications: bool,
    pub autostart: bool,
    pub refresh_interval_secs: u32,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            notifications: true,
            autostart: false,
            refresh_interval_secs: 4,
        }
    }
}

impl Prefs {
    /// Clamp the polling interval to a sane range.
    pub fn interval_secs(&self) -> u64 {
        self.refresh_interval_secs.clamp(2, 60) as u64
    }
}

fn prefs_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("prefs.json"))
}

/// Load preferences, falling back to defaults on any error.
pub fn load(app: &AppHandle) -> Prefs {
    match prefs_path(app).ok().and_then(|p| fs::read_to_string(p).ok()) {
        Some(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        None => Prefs::default(),
    }
}

/// Persist preferences to disk, creating the config directory if needed.
pub fn save(app: &AppHandle, prefs: &Prefs) -> Result<(), String> {
    let path = prefs_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}
