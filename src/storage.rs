use std::ffi::{c_void, OsString};
use std::fs;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::process::Command;

use crate::model::{AppConfig, MouseProfile};

const OFN_OVERWRITEPROMPT: u32 = 0x0000_0002;
const OFN_NOCHANGEDIR: u32 = 0x0000_0008;
const OFN_PATHMUSTEXIST: u32 = 0x0000_0800;
const OFN_FILEMUSTEXIST: u32 = 0x0000_1000;
const OFN_EXPLORER: u32 = 0x0008_0000;

#[repr(C)]
struct OPENFILENAMEW {
    l_struct_size: u32,
    hwnd_owner: *mut c_void,
    h_instance: *mut c_void,
    lpstr_filter: *const u16,
    lpstr_custom_filter: *mut u16,
    n_max_cust_filter: u32,
    n_filter_index: u32,
    lpstr_file: *mut u16,
    n_max_file: u32,
    lpstr_file_title: *mut u16,
    n_max_file_title: u32,
    lpstr_initial_dir: *const u16,
    lpstr_title: *const u16,
    flags: u32,
    n_file_offset: u16,
    n_file_extension: u16,
    lpstr_def_ext: *const u16,
    l_cust_data: isize,
    lpfn_hook: *mut c_void,
    lp_template_name: *const u16,
    pv_reserved: *mut c_void,
    dw_reserved: u32,
    flags_ex: u32,
}

#[cfg(target_pointer_width = "64")]
const _: () = assert!(size_of::<OPENFILENAMEW>() == 152);

#[link(name = "comdlg32")]
unsafe extern "system" {
    fn GetOpenFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
    fn GetSaveFileNameW(ofn: *mut OPENFILENAMEW) -> i32;
}

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn run_file_dialog(save: bool, title: &str, default_name: Option<&str>) -> Option<PathBuf> {
    let filter: Vec<u16> = "ATLAS Profile JSON\0*.json\0All Files\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title_w = to_wide_null(title);
    let def_ext_w = to_wide_null("json");

    let mut file_buf = vec![0u16; 4096];
    if let Some(name) = default_name {
        for (i, ch) in name.encode_utf16().take(file_buf.len() - 1).enumerate() {
            file_buf[i] = ch;
        }
    }

    unsafe {
        let mut ofn: OPENFILENAMEW = zeroed();
        ofn.l_struct_size = size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstr_filter = filter.as_ptr();
        ofn.n_filter_index = 1;
        ofn.lpstr_file = file_buf.as_mut_ptr();
        ofn.n_max_file = file_buf.len() as u32;
        ofn.lpstr_title = title_w.as_ptr();
        ofn.lpstr_def_ext = def_ext_w.as_ptr();
        ofn.flags = if save {
            OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_OVERWRITEPROMPT | OFN_NOCHANGEDIR
        } else {
            OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR
        };

        let ok = if save {
            GetSaveFileNameW(&mut ofn)
        } else {
            GetOpenFileNameW(&mut ofn)
        };
        if ok == 0 {
            return None;
        }
    }

    let len = file_buf
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(file_buf.len());
    if len == 0 {
        return None;
    }
    Some(PathBuf::from(OsString::from_wide(&file_buf[..len])))
}

pub fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | ' ' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim_matches(['_', '.']);
    if trimmed.is_empty() {
        "ATLAS_Profile".into()
    } else {
        trimmed.into()
    }
}

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
    cfg.sanitize();
    cfg
}

pub fn save_config(cfg: &AppConfig) -> Result<(), String> {
    let path = config_file_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Gagal membuat folder config: {e}"))?;
    }
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("Gagal menyimpan format config: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Gagal menulis file config: {e}"))?;
    Ok(())
}

pub fn export_profile_dialog(profile: &MouseProfile) -> Result<Option<PathBuf>, String> {
    let suggested = format!("{}.atlas.json", sanitize_filename(&profile.name));
    let Some(path) = run_file_dialog(true, "Ekspor Profil ATLAS", Some(&suggested)) else {
        return Ok(None);
    };
    let json = serde_json::to_string_pretty(profile)
        .map_err(|e| format!("Gagal membuat JSON profil: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Gagal menyimpan profil: {e}"))?;
    Ok(Some(path))
}

pub fn import_profile_dialog() -> Result<Option<MouseProfile>, String> {
    let Some(path) = run_file_dialog(false, "Impor Profil ATLAS", None) else {
        return Ok(None);
    };
    let raw = fs::read_to_string(&path).map_err(|e| format!("Gagal membaca file: {e}"))?;
    let mut prof: MouseProfile =
        serde_json::from_str(&raw).map_err(|e| format!("File JSON profil tidak valid: {e}"))?;
    prof.sanitize();
    Ok(Some(prof))
}

pub fn open_windows_mouse_properties() {
    let _ = Command::new("control").arg("main.cpl").spawn();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("FPS / Valorant: Pro*"), "FPS___Valorant__Pro");
        assert_eq!(sanitize_filename("   ...  "), "ATLAS_Profile");
        assert_eq!(sanitize_filename("ATLAS Default"), "ATLAS_Default");
    }

    #[test]
    fn test_profile_sanitize_repairs_bad_values() {
        let mut prof = MouseProfile::factory_default("");
        for st in &mut prof.dpi_stages {
            st.enabled = false;
            st.dpi = 999_999;
        }
        prof.active_dpi_stage = 99;
        prof.debounce_ms = 0;
        prof.power.sleep_minutes = 0;
        prof.power.deep_sleep_minutes = 200;
        prof.buttons[0] = crate::model::ButtonAction::Disabled;

        prof.sanitize();

        assert_eq!(prof.name, "ATLAS Profile");
        assert!(prof.dpi_stages[0].enabled);
        assert_eq!(prof.dpi_stages[0].dpi, 24000);
        assert_eq!(prof.active_dpi_stage, 0);
        assert_eq!(prof.debounce_ms, 2);
        assert_eq!(prof.power.sleep_minutes, 1);
        assert_eq!(prof.power.deep_sleep_minutes, 60);
        assert_eq!(prof.count_left_clicks(), 1);
    }
}
