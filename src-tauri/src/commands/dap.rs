use crate::dap::client::{DapManager, DapServerConfig, SourceBreakpoint};
use crate::dap::servers::{DapServerManager, InstalledDapServer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use once_cell::sync::Lazy;
use tokio::sync::Mutex as TokioMutex;
use tracing::{info, warn};

static DAP_MANAGER: Lazy<Arc<TokioMutex<Option<DapManager>>>> = Lazy::new(|| {
    Arc::new(TokioMutex::new(None))
});

static DAP_SERVER_MANAGER: Lazy<Arc<TokioMutex<Option<DapServerManager>>>> = Lazy::new(|| {
    Arc::new(TokioMutex::new(None))
});

static DAP_CONFIGS: Lazy<Arc<TokioMutex<HashMap<String, DapServerConfig>>>> = Lazy::new(|| {
    let mut configs = HashMap::new();

    // Rust/C++ - CodeLLDB
    configs.insert("rust".to_string(), DapServerConfig {
        name: "codelldb".to_string(),
        command: vec!["codelldb".to_string()],
        languages: vec!["rust".to_string(), "cpp".to_string(), "c".to_string()],
        file_extensions: vec!["rs".to_string(), "cpp".to_string(), "cc".to_string(), "c".to_string()],
        download_url: Some("https://github.com/vadimcn/codelldb/releases/latest/download/codelldb-x86_64-linux.vsix".to_string()),
        args: vec!["--port".to_string(), "0".to_string()],
    });

    // TypeScript/JavaScript - node-debug2
    configs.insert("typescript".to_string(), DapServerConfig {
        name: "node-debug2".to_string(),
        command: vec!["node".to_string()],
        languages: vec!["typescript".to_string(), "javascript".to_string(), "typescriptreact".to_string(), "javascriptreact".to_string()],
        file_extensions: vec!["ts".to_string(), "js".to_string(), "tsx".to_string(), "jsx".to_string()],
        download_url: None,
        args: vec!["--inspect-brk".to_string()],
    });

    // Python - debugpy
    configs.insert("python".to_string(), DapServerConfig {
        name: "debugpy".to_string(),
        command: vec!["python".to_string(), "-m".to_string(), "debugpy.adapter".to_string()],
        languages: vec!["python".to_string()],
        file_extensions: vec!["py".to_string()],
        download_url: None,
        args: vec!["--host".to_string(), "127.0.0.1".to_string(), "--port".to_string(), "0".to_string()],
    });

    // Go - delve
    configs.insert("go".to_string(), DapServerConfig {
        name: "dlv".to_string(),
        command: vec!["dlv".to_string(), "dap".to_string()],
        languages: vec!["go".to_string()],
        file_extensions: vec!["go".to_string()],
        download_url: None,
        args: vec!["--listen".to_string(), "127.0.0.1:0".to_string()],
    });

    Arc::new(TokioMutex::new(configs))
});

async fn get_dap_manager() -> Result<DapManager, String> {
    let mut manager_guard = DAP_MANAGER.lock().await;
    if manager_guard.is_none() {
        let (manager, _rx) = DapManager::new();
        *manager_guard = Some(manager);
    }
    Ok(manager_guard.as_ref().unwrap().clone())
}

async fn get_dap_server_manager() -> Result<DapServerManager, String> {
    let mut manager_guard = DAP_SERVER_MANAGER.lock().await;
    if manager_guard.is_none() {
        *manager_guard = Some(DapServerManager::new().map_err(|e| e.to_string())?);
    }
    Ok(manager_guard.as_ref().unwrap().clone())
}

#[tauri::command]
pub async fn dap_initialize(app: AppHandle, workspace_root: String) -> Result<String, String> {
    let workspace = PathBuf::from(&workspace_root);
    let configs = DAP_CONFIGS.lock().await;
    let server_manager = get_dap_server_manager().await?;

    for (lang, config) in configs.iter() {
        let binary_path = server_manager.ensure_server(config).await
            .map_err(|e| format!("Failed to install {}: {}", config.name, e))?;

        let manager = get_dap_manager().await?;
        manager.get_or_create_client(
            workspace.clone(),
            &config.name,
            config.command.clone(),
            config.args.clone(),
        ).await.map_err(|e| format!("Failed to start {}: {}", config.name, e))?;
    }

    Ok("DAP servers initialized".to_string())
}

#[tauri::command]
pub async fn dap_shutdown() -> Result<String, String> {
    let manager = get_dap_manager().await?;
    manager.shutdown_all().await.map_err(|e| e.to_string())?;
    Ok("DAP servers shutdown".to_string())
}

#[tauri::command]
pub async fn dap_launch(
    uri: String,
    program: String,
    args: Vec<String>,
    cwd: String,
    env: HashMap<String, String>,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    // Find the appropriate DAP client for this file
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        PathBuf::from(&cwd),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.launch(&program, args, PathBuf::from(cwd), env).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_attach(
    uri: String,
    process_id: u32,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.attach(process_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_set_breakpoints(
    uri: String,
    breakpoints: Vec<SourceBreakpoint>,
) -> Result<Vec<crate::dap::client::Breakpoint>, String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    let path = url.to_file_path().map_err(|_| "Invalid file path".to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        path.parent().unwrap().to_path_buf(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.set_breakpoints(&path.to_string_lossy(), breakpoints).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_continue(
    uri: String,
    thread_id: u64,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.continue_execution(thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_pause(
    uri: String,
    thread_id: u64,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.pause(thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_step_over(
    uri: String,
    thread_id: u64,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.step_over(thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_step_into(
    uri: String,
    thread_id: u64,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.step_into(thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_step_out(
    uri: String,
    thread_id: u64,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.step_out(thread_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_get_threads(
    uri: String,
) -> Result<Vec<crate::dap::client::Thread>, String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.get_threads().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_get_stack_trace(
    uri: String,
    thread_id: u64,
    start_frame: u32,
    levels: u32,
) -> Result<Vec<crate::dap::client::StackFrame>, String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.get_stack_trace(thread_id, start_frame, levels).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_get_scopes(
    uri: String,
    frame_id: u64,
) -> Result<Vec<crate::dap::client::Scope>, String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.get_scopes(frame_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_get_variables(
    uri: String,
    variables_reference: u64,
) -> Result<Vec<crate::dap::client::Variable>, String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.get_variables(variables_reference).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_evaluate(
    uri: String,
    expression: String,
    frame_id: Option<u64>,
    context: String,
) -> Result<crate::dap::client::EvaluateResult, String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.evaluate(&expression, frame_id, &context).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_disconnect(
    uri: String,
    terminate_debuggee: bool,
) -> Result<(), String> {
    let url = url::Url::parse(&uri).map_err(|e| e.to_string())?;
    
    let configs = DAP_CONFIGS.lock().await;
    let lang = detect_language_from_uri(&url);
    let config = configs.get(&lang).ok_or_else(|| format!("No DAP config for language: {}", lang))?.clone();
    drop(configs);
    
    let manager = get_dap_manager().await?;
    let client = manager.get_or_create_client(
        std::env::current_dir().unwrap(),
        &config.name,
        config.command.clone(),
        config.args.clone(),
    ).await.map_err(|e| e.to_string())?;
    
    let mut client_guard = client.lock().await;
    client_guard.disconnect(terminate_debuggee).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dap_install_server(language: String) -> Result<String, String> {
    let configs = DAP_CONFIGS.lock().await;
    let config = configs.get(&language).ok_or_else(|| format!("Unknown language: {}", language))?.clone();
    drop(configs);
    
    let server_manager = get_dap_server_manager().await?;
    server_manager.ensure_server(&config).await.map_err(|e| e.to_string())?;
    
    Ok(format!("Installed {}", config.name))
}

#[tauri::command]
pub async fn dap_uninstall_server(name: String) -> Result<String, String> {
    let server_manager = get_dap_server_manager().await?;
    server_manager.uninstall_server(&name).await.map_err(|e| e.to_string())?;
    Ok(format!("Uninstalled {}", name))
}

#[tauri::command]
pub async fn dap_get_installed_servers() -> Result<Vec<InstalledDapServer>, String> {
    let server_manager = get_dap_server_manager().await?;
    Ok(server_manager.get_installed_servers().into_values().collect())
}

#[tauri::command]
pub async fn dap_get_configs() -> Result<Vec<DapConfigInfo>, String> {
    let configs = DAP_CONFIGS.lock().await;
    let server_manager = get_dap_server_manager().await.ok();
    let installed = server_manager.map(|m| m.get_installed_servers()).unwrap_or_default();
    
    let mut result = Vec::new();

    for (lang, config) in configs.iter() {
        let installed_server = installed.get(&config.name);
        result.push(DapConfigInfo {
            language: lang.clone(),
            name: config.name.clone(),
            command: config.command.clone(),
            args: config.args.clone(),
            file_extensions: config.file_extensions.clone(),
            available: installed_server.is_some(),
            installed_version: installed_server.map(|s| s.version.clone()),
            installed_path: installed_server.map(|s| s.path.display().to_string()),
        });
    }

    Ok(result)
}

#[derive(serde::Serialize)]
pub struct DapConfigInfo {
    pub language: String,
    pub name: String,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
    pub available: bool,
    pub installed_version: Option<String>,
    pub installed_path: Option<String>,
}

fn detect_language_from_uri(uri: &url::Url) -> String {
    let path = uri.to_file_path().ok();
    if let Some(path) = path {
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext {
                "rs" => return "rust".to_string(),
                "ts" | "tsx" => return "typescript".to_string(),
                "js" | "jsx" => return "typescript".to_string(),
                "py" => return "python".to_string(),
                "go" => return "go".to_string(),
                "cpp" | "cc" | "cxx" | "c" => return "rust".to_string(), // Use CodeLLDB for C/C++
                _ => {}
            }
        }
    }
    "typescript".to_string() // Default
}