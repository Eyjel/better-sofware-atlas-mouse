use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::model::{AppConfig, MouseProfile};

pub fn config_file_path() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("AtlasControl").join("config.json")
}

pub fn load_config() -> AppConfig {
    let path = config_file_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return AppConfig::default();
    };
    let mut cfg: AppConfig = serde_json::from_str(&raw).unwrap_or_default();
    if cfg.profiles.is_empty() {
        cfg.profiles.push(MouseProfile::factory_default("ATLAS Default"));
    }
    if cfg.active_profile_idx >= cfg.profiles.len() {
        cfg.active_profile_idx = 0;
    }
    cfg
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write config: {e}"))?;
    Ok(())
}

pub fn export_profile_dialog(profile: &MouseProfile) -> Result<Option<PathBuf>, String> {
    let suggested = format!("{}.atlas.json", profile.name.replace(' ', "_"));
    let Some(path) = rfd::FileDialog::new()
        .set_title("Export ATLAS Profile")
        .set_file_name(&suggested)
        .add_filter("ATLAS Profile JSON", &["json"])
        .save_file()
    else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Failed to serialize profile: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Failed to save profile: {e}"))?;
    Ok(Some(path))
}

pub fn import_profile_dialog() -> Result<Option<MouseProfile>, String> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Import ATLAS Profile")
        .add_filter("ATLAS Profile JSON", &["json"])
        .pick_file()
    else {
        return Ok(None);
    };
    let raw = fs::read_to_string(&path).map_err(|e| format!("Failed to read file: {e}"))?;
    let prof: MouseProfile =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid profile JSON: {e}"))?;
    Ok(Some(prof))
}

pub fn open_windows_mouse_properties() {
    let _ = Command::new("control").arg("main.cpl").spawn();
}
