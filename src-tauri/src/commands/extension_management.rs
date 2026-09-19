use crate::services::extension_manager::{ExtensionManager, InstalledExtension, SearchResult};
use tracing::info;

static EXTENSION_MANAGER: std::sync::LazyLock<std::sync::Mutex<Option<ExtensionManager>>> = 
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

fn get_extension_manager() -> Result<ExtensionManager, String> {
    let mut manager_guard = EXTENSION_MANAGER.lock().unwrap();
    if manager_guard.is_none() {
        *manager_guard = Some(ExtensionManager::new().map_err(|e| e.to_string())?);
    }
    Ok(manager_guard.as_ref().unwrap().clone())
}

#[tauri::command]
pub async fn extension_search(query: String, page: Option<u32>, page_size: Option<u32>) -> Result<SearchResult, String> {
    let manager = get_extension_manager()?;
    let page = page.unwrap_or(1);
    let page_size = page_size.unwrap_or(20);
    manager.search_extensions(&query, page, page_size).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extension_get_info(publisher: String, name: String) -> Result<serde_json::Value, String> {
    let manager = get_extension_manager()?;
    let info = manager.get_extension_info(&publisher, &name).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(info).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub async fn extension_install(publisher: String, name: String, version: Option<String>) -> Result<InstalledExtension, String> {
    let manager = get_extension_manager()?;
    let version_ref = version.as_deref();
    manager.install_extension(&publisher, &name, version_ref).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extension_uninstall(publisher: String, name: String) -> Result<String, String> {
    let manager = get_extension_manager()?;
    manager.uninstall_extension(&publisher, &name).await.map_err(|e| e.to_string())?;
    Ok(format!("Uninstalled {}.{}", publisher, name))
}

#[tauri::command]
pub async fn extension_enable(publisher: String, name: String) -> Result<String, String> {
    let manager = get_extension_manager()?;
    manager.enable_extension(&publisher, &name).await.map_err(|e| e.to_string())?;
    Ok(format!("Enabled {}.{}", publisher, name))
}

#[tauri::command]
pub async fn extension_disable(publisher: String, name: String) -> Result<String, String> {
    let manager = get_extension_manager()?;
    manager.disable_extension(&publisher, &name).await.map_err(|e| e.to_string())?;
    Ok(format!("Disabled {}.{}", publisher, name))
}

#[tauri::command]
pub async fn extension_list_installed() -> Result<Vec<InstalledExtension>, String> {
    let manager = get_extension_manager()?;
    Ok(manager.get_installed_extensions())
}

#[tauri::command]
pub async fn extension_get_installed(publisher: String, name: String) -> Result<Option<InstalledExtension>, String> {
    let manager = get_extension_manager()?;
    Ok(manager.get_extension(&publisher, &name))
}

#[tauri::command]
pub async fn extension_get_activation_events(publisher: String, name: String) -> Result<Vec<String>, String> {
    let manager = get_extension_manager()?;
    Ok(manager.get_activation_events(&publisher, &name))
}

#[tauri::command]
pub async fn extension_activate(publisher: String, name: String) -> Result<String, String> {
    // Activate through the extension host
    // This would use the extension host channel to activate
    info!("Activating extension {}.{}", publisher, name);
    Ok(format!("Activation requested for {}.{}", publisher, name))
}

#[tauri::command]
pub async fn extension_deactivate(publisher: String, name: String) -> Result<String, String> {
    info!("Deactivating extension {}.{}", publisher, name);
    Ok(format!("Deactivation requested for {}.{}", publisher, name))
}