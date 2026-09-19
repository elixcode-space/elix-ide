use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::UpdaterExt;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(serde::Serialize, Clone)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
}

#[derive(serde::Serialize, Clone)]
pub struct UpdateProgress {
    pub status: String,
    pub progress: Option<f64>,
    pub message: String,
}

pub struct UpdaterState {
    pub current_version: String,
    pub update_check_in_progress: Arc<Mutex<bool>>,
}

impl UpdaterState {
    pub fn new() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            update_check_in_progress: Arc::new(Mutex::new(false)),
        }
    }
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    let state = app.state::<UpdaterState>();
    let mut in_progress = state.update_check_in_progress.lock().await;
    
    if *in_progress {
        return Err("Update check already in progress".to_string());
    }
    *in_progress = true;
    drop(in_progress);

    let result = async {
        let updater = app.updater()
            .map_err(|e| format!("Failed to get updater: {}", e))?;

        let update = updater.check().await
            .map_err(|e| format!("Failed to check for updates: {}", e))?;

        let info = match update {
            Some(update) => UpdateInfo {
                available: true,
                current_version: state.current_version.clone(),
                latest_version: Some(update.version.clone()),
                notes: update.body,
                pub_date: update.date.map(|d| d.to_string()),
            },
            None => UpdateInfo {
                available: false,
                current_version: state.current_version.clone(),
                latest_version: None,
                notes: None,
                pub_date: None,
            },
        };

        let mut in_progress = state.update_check_in_progress.lock().await;
        *in_progress = false;

        Ok(info)
    }.await;

    if result.is_err() {
        let mut in_progress = state.update_check_in_progress.lock().await;
        *in_progress = false;
    }

    result
}

#[tauri::command]
pub async fn download_and_install_update(
    app: AppHandle,
    on_progress: tauri::ipc::Channel<UpdateProgress>,
) -> Result<String, String> {
    let updater = app.updater()
        .map_err(|e| format!("Failed to get updater: {}", e))?;

    let update = updater.check().await
        .map_err(|e| format!("Failed to check for updates: {}", e))?
        .ok_or("No update available")?;

    on_progress.send(UpdateProgress {
        status: "downloading".to_string(),
        progress: Some(0.0),
        message: "Downloading update...".to_string(),
    }).ok();

    let mut downloaded = 0u64;

    let _update = update.download_and_install(
        |chunk_length, content_length| {
            downloaded += chunk_length as u64;
            let progress = content_length.map(|cl| downloaded as f64 / cl as f64);
            
            let _ = on_progress.send(UpdateProgress {
                status: "downloading".to_string(),
                progress,
                message: format!("Downloaded {} bytes", downloaded),
            });
        },
        || {
            let _ = on_progress.send(UpdateProgress {
                status: "installing".to_string(),
                progress: Some(1.0),
                message: "Installing update...".to_string(),
            });
        },
    ).await
    .map_err(|e| format!("Failed to download and install update: {}", e))?;

    on_progress.send(UpdateProgress {
        status: "completed".to_string(),
        progress: Some(1.0),
        message: "Update installed successfully. Restart to apply.".to_string(),
    }).ok();

    Ok("Update installed. Please restart the application.".to_string())
}

#[tauri::command]
pub async fn install_update_and_restart(app: AppHandle) -> Result<(), String> {
    let updater = app.updater()
        .map_err(|e| format!("Failed to get updater: {}", e))?;

    let update = updater.check().await
        .map_err(|e| format!("Failed to check for updates: {}", e))?
        .ok_or("No update available")?;

    update.download_and_install(|_, _| {}, || {}).await
        .map_err(|e| format!("Failed to download and install update: {}", e))?;

    app.restart();
    Ok(())
}

#[tauri::command]
pub fn get_current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn set_update_channel(_app: AppHandle, channel: String) -> Result<String, String> {
    // Update channel switching would require modifying the updater config
    // This is a placeholder for future implementation
    Ok(format!("Update channel set to: {}. Restart required.", channel))
}