use crate::terminal::service::TerminalService;
use crate::terminal::types::*;
use tauri::command;
use std::sync::Arc;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    static ref TERMINAL_SERVICE: Arc<Mutex<TerminalService>> = {
        let (service, mut rx) = TerminalService::new();
        let service = Arc::new(Mutex::new(service));
        
        let service_clone = service.clone();
        tokio::spawn(async move {
            while let Some((id, event)) = rx.recv().await {
                // Emit events to frontend via Tauri
                // This would use Tauri's event system
                println!("Terminal event for {}: {:?}", id, event);
            }
        });
        
        service
    };
}

pub struct TerminalState {
    pub service: Arc<Mutex<TerminalService>>,
}

impl TerminalState {
    pub fn new() -> Self {
        Self {
            service: TERMINAL_SERVICE.clone(),
        }
    }
}

impl Default for TerminalState {
    fn default() -> Self {
        Self::new()
    }
}

#[command]
pub async fn terminal_create(options: TerminalCreateOptions) -> Result<TerminalInstance, String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.create_terminal(options).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_get(id: String) -> Result<TerminalInstance, String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.get_terminal(&id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_list() -> Result<Vec<TerminalInstance>, String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.list_terminals().await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_write(id: String, options: TerminalWriteOptions) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.write(&id, options.data).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_resize(id: String, options: TerminalResizeOptions) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.resize(&id, options.cols, options.rows).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_kill(id: String) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.kill(&id).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_set_title(id: String, title: String) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.set_title(&id, title).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_get_profiles() -> Result<Vec<TerminalProfile>, String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.get_profiles().await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_add_profile(profile: TerminalProfile) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.add_profile(profile).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_update_profile(name: String, profile: TerminalProfile) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.update_profile(&name, profile).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_delete_profile(name: String) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.delete_profile(&name).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_get_config() -> Result<TerminalConfig, String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.get_config().await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_update_config(config: TerminalConfig) -> Result<(), String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.update_config(config).await.map_err(|e| e.to_string())
}

#[command]
pub async fn terminal_detect_links(id: String, text: String, line: u32) -> Result<Vec<TerminalLink>, String> {
    let service = TERMINAL_SERVICE.lock().await;
    Ok(service.detect_links(&id, &text, line).await)
}

#[command]
pub async fn terminal_get_process_info(id: String) -> Result<Vec<TerminalProcessInfo>, String> {
    let service = TERMINAL_SERVICE.lock().await;
    service.get_process_info(&id).await.map_err(|e| e.to_string())
}