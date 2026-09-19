use crate::config::settings::Settings;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use once_cell::sync::Lazy;

pub struct SettingsState {
    settings: Mutex<Settings>,
    path: PathBuf,
}

static SETTINGS_STATE: Lazy<Arc<SettingsState>> = Lazy::new(|| {
    let app_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("elixide");
    let path = app_dir.join("settings.json");
    
    let settings = if path.exists() {
        Settings::load_from_file(&path).unwrap_or_default()
    } else {
        Settings::default()
    };
    
    Arc::new(SettingsState {
        settings: Mutex::new(settings),
        path,
    })
});

#[tauri::command]
pub fn get_settings() -> Result<Settings, String> {
    let state = &*SETTINGS_STATE;
    let settings = state.settings.lock().unwrap();
    Ok(settings.clone())
}

#[tauri::command]
pub fn update_settings(partial: serde_json::Value) -> Result<Settings, String> {
    let state = &*SETTINGS_STATE;
    let mut settings = state.settings.lock().unwrap();
    
    // Merge partial settings
    let current_json = serde_json::to_value(&*settings).map_err(|e| e.to_string())?;
    let merged = json_merge(current_json, partial);
    *settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    
    // Save to file
    settings.save_to_file(&state.path).map_err(|e| e.to_string())?;
    
    Ok(settings.clone())
}

#[tauri::command]
pub fn reset_settings() -> Result<Settings, String> {
    let state = &*SETTINGS_STATE;
    let mut settings = state.settings.lock().unwrap();
    *settings = Settings::default();
    settings.save_to_file(&state.path).map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn get_settings_path() -> Result<String, String> {
    let state = &*SETTINGS_STATE;
    Ok(state.path.to_string_lossy().to_string())
}

fn json_merge(mut base: serde_json::Value, overlay: serde_json::Value) -> serde_json::Value {
    if let (serde_json::Value::Object(base_obj), serde_json::Value::Object(overlay_obj)) = (&mut base, &overlay) {
        for (key, value) in overlay_obj {
            if let Some(base_value) = base_obj.get_mut(key) {
                *base_value = json_merge(base_value.clone(), value.clone());
            } else {
                base_obj.insert(key.clone(), value.clone());
            }
        }
    } else {
        return overlay;
    }
    base
}

#[tauri::command]
pub async fn open_settings_file(app: AppHandle) -> Result<(), String> {
    let state = &*SETTINGS_STATE;
    let path = &state.path;
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}