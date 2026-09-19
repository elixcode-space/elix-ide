use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::collections::HashMap;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use tauri::{Emitter, Manager, Window};
use serde_json::Value;

use piscis_kernel::headless::{open_kernel_state, register_default_cli_tools, run_piscis_turn, HeadlessDeps, KernelState};
use piscis_core::host::{EventSink, ToolRegistryHandle, HeadlessCliRequest, HeadlessCliMode};
use piscis_kernel::agent::tool::ToolRegistry;

pub struct CoreEngineState {
    kernel_state: Option<KernelState>,
    app_data_dir: Option<PathBuf>,
    session_cache: StdMutex<HashMap<String, String>>,
}

impl CoreEngineState {
    pub fn new() -> Self {
        Self {
            kernel_state: None,
            app_data_dir: None,
            session_cache: StdMutex::new(HashMap::new()),
        }
    }
}

static CORE_ENGINE: Lazy<Mutex<CoreEngineState>> = Lazy::new(|| Mutex::new(CoreEngineState::new()));

pub fn create_tool_registry(db: &Arc<Mutex<piscis_kernel::store::db::Database>>, settings: &Arc<Mutex<piscis_kernel::store::settings::Settings>>) -> ToolRegistry {
    let mut tool_registry = ToolRegistry::new();
    let mut handle = ToolRegistryHandle::new(tool_registry);
    register_default_cli_tools(&mut handle, db.clone(), settings.clone());
    match handle.into_inner() {
        Ok(reg) => reg,
        Err(_) => ToolRegistry::new(), // fallback
    }
}

pub async fn get_kernel_state() -> Option<(Arc<Mutex<piscis_kernel::store::db::Database>>, Arc<Mutex<piscis_kernel::store::settings::Settings>>)> {
    let state = CORE_ENGINE.lock().await;
    state.kernel_state.clone()
}

struct TauriEventSink {
    window: Window,
    session_id: String,
}

impl EventSink for TauriEventSink {
    fn emit_session(&self, session_id: &str, event: &str, payload: Value) {
        let event_name = format!("core-{}", event);
        let _ = self.window.emit(event_name.as_str(), serde_json::json!({
            "session_id": session_id,
            "event": event,
            "payload": payload
        }));
    }

    fn emit_broadcast(&self, event: &str, payload: Value) {
        let _ = self.window.emit("core-broadcast", serde_json::json!({
            "event": event,
            "payload": payload
        }));
    }
}

pub async fn initialize(app_handle: tauri::AppHandle) -> Result<String, String> {
    let mut state = CORE_ENGINE.lock().await;

    if state.kernel_state.is_some() {
        return Ok("Core engine already initialized".to_string());
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let kernel_state = open_kernel_state(&app_data_dir)
        .map_err(|e| format!("Failed to open kernel state: {}", e))?;

    state.kernel_state = Some(kernel_state);
    state.app_data_dir = Some(app_data_dir);

    Ok("Core engine initialized with piscis-engine".to_string())
}

pub async fn run_query(query: String, window: Window) -> Result<String, String> {
    let state = CORE_ENGINE.lock().await;

    let kernel_state = match &state.kernel_state {
        Some(ks) => ks.clone(),
        _ => return Err("Core engine not initialized. Call initialize_core first.".to_string()),
    };

    let (db, settings) = &kernel_state;
    let tool_registry = create_tool_registry(db, settings);

    let session_id = format!("session-{}", uuid::Uuid::new_v4().simple());
    let event_sink = Arc::new(TauriEventSink { window, session_id: session_id.clone() });

    let deps = HeadlessDeps::new(
        db.clone(),
        settings.clone(),
        tool_registry,
        event_sink,
    );

    let request = HeadlessCliRequest {
        prompt: query,
        workspace: state.app_data_dir.as_ref().map(|p| p.to_string_lossy().to_string()),
        mode: HeadlessCliMode::Piscis,
        session_id: Some(session_id.clone()),
        ..Default::default()
    };

    let response = run_piscis_turn(request, deps)
        .await
        .map_err(|e| format!("Agent query failed: {}", e))?;

    Ok(response.response_text)
}

pub async fn stream_query(query: String, window: Window) -> Result<(), String> {
    let state = CORE_ENGINE.lock().await;

    let kernel_state = match &state.kernel_state {
        Some(ks) => ks.clone(),
        _ => return Err("Core engine not initialized. Call initialize_core first.".to_string()),
    };

    let (db, settings) = &kernel_state;
    let tool_registry = create_tool_registry(db, settings);

    let session_id = format!("session-{}", uuid::Uuid::new_v4().simple());
    let event_sink = Arc::new(TauriEventSink { window: window.clone(), session_id: session_id.clone() });

    let deps = HeadlessDeps::new(
        db.clone(),
        settings.clone(),
        tool_registry,
        event_sink,
    );

    let request = HeadlessCliRequest {
        prompt: query,
        workspace: state.app_data_dir.as_ref().map(|p| p.to_string_lossy().to_string()),
        mode: HeadlessCliMode::Piscis,
        session_id: Some(session_id.clone()),
        ..Default::default()
    };

    let _ = window.emit("core-response-start", serde_json::json!({"session_id": session_id}));

    let response = run_piscis_turn(request, deps)
        .await
        .map_err(|e| format!("Agent stream failed: {}", e))?;

    let _ = window.emit("core-response-end", serde_json::json!({
        "session_id": session_id,
        "response": response.response_text,
        "ok": response.ok
    }));

    Ok(())
}

pub async fn get_session_id(key: &str) -> Option<String> {
    let state = CORE_ENGINE.lock().await;
    let result = state.session_cache.lock().unwrap().get(key).cloned();
    result
}

pub async fn set_session_id(key: &str, session_id: String) {
    let state = CORE_ENGINE.lock().await;
    state.session_cache.lock().unwrap().insert(key.to_string(), session_id);
}