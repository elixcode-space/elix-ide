use crate::lsp::client::{LSPManager, LSPServerConfig};
use crate::lsp::servers::{LSPServerManager, InstalledServer, ServerUpdate};
use crate::config::settings::LSPSettings;
use lsp_types::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use once_cell::sync::Lazy;
use tokio::sync::Mutex as TokioMutex;
use tracing::{info, warn, error};

static LSP_MANAGER: Lazy<Arc<TokioMutex<LSPManager>>> = Lazy::new(|| {
    Arc::new(TokioMutex::new(LSPManager::new()))
});

static LSP_SERVER_MANAGER: Lazy<Arc<TokioMutex<Option<LSPServerManager>>>> = Lazy::new(|| {
    Arc::new(TokioMutex::new(None))
});

static LSP_CONFIGS: Lazy<Arc<TokioMutex<HashMap<String, LSPServerConfig>>>> = Lazy::new(|| {
    let mut configs = HashMap::new();

    // Rust
    configs.insert("rust".to_string(), LSPServerConfig {
        name: "rust-analyzer".to_string(),
        command: vec!["rust-analyzer".to_string()],
        languages: vec!["rust".to_string()],
        file_extensions: vec!["rs".to_string()],
        root_patterns: vec!["Cargo.toml".to_string()],
        download_url: Some("https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu.gz".to_string()),
    });

    // TypeScript/JavaScript
    configs.insert("typescript".to_string(), LSPServerConfig {
        name: "typescript-language-server".to_string(),
        command: vec!["typescript-language-server".to_string(), "--stdio".to_string()],
        languages: vec!["typescript".to_string(), "javascript".to_string(), "typescriptreact".to_string(), "javascriptreact".to_string()],
        file_extensions: vec!["ts".to_string(), "js".to_string(), "tsx".to_string(), "jsx".to_string()],
        root_patterns: vec!["package.json".to_string(), "tsconfig.json".to_string()],
        download_url: None,
    });

    // Python
    configs.insert("python".to_string(), LSPServerConfig {
        name: "pyright-langserver".to_string(),
        command: vec!["pyright-langserver".to_string(), "--stdio".to_string()],
        languages: vec!["python".to_string()],
        file_extensions: vec!["py".to_string()],
        root_patterns: vec!["pyproject.toml".to_string(), "setup.py".to_string(), "requirements.txt".to_string()],
        download_url: None,
    });

    // Go
    configs.insert("go".to_string(), LSPServerConfig {
        name: "gopls".to_string(),
        command: vec!["gopls".to_string()],
        languages: vec!["go".to_string()],
        file_extensions: vec!["go".to_string()],
        root_patterns: vec!["go.mod".to_string()],
        download_url: None,
    });

    Arc::new(TokioMutex::new(configs))
});

async fn get_server_manager() -> Result<LSPServerManager, String> {
    let mut manager_guard = LSP_SERVER_MANAGER.lock().await;
    if manager_guard.is_none() {
        *manager_guard = Some(LSPServerManager::new().map_err(|e| e.to_string())?);
    }
    Ok(manager_guard.as_ref().unwrap().clone())
}

#[tauri::command]
pub async fn lsp_initialize(app: AppHandle, workspace_root: String) -> Result<String, String> {
    let workspace = PathBuf::from(&workspace_root);
    let configs = LSP_CONFIGS.lock().await;
    let server_manager = get_server_manager().await?;

    for (lang, config) in configs.iter() {
        // Ensure server is installed
        let binary_path = server_manager.ensure_server(config).await
            .map_err(|e| format!("Failed to install {}: {}", config.name, e))?;

        let manager = LSP_MANAGER.lock().await;
        manager.get_or_create_client(workspace.clone(), &config.name, config.command.clone()).await
            .map_err(|e| format!("Failed to start {}: {}", config.name, e))?;
    }

    Ok("LSP servers initialized".to_string())
}

#[tauri::command]
pub async fn lsp_shutdown() -> Result<String, String> {
    let manager = LSP_MANAGER.lock().await;
    manager.shutdown_all().await.map_err(|e| e.to_string())?;
    Ok("LSP servers shutdown".to_string())
}

#[tauri::command]
pub async fn lsp_completion(
    uri: String,
    line: u32,
    character: u32,
) -> Result<Option<CompletionResponse>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let position = Position { line, character };

    let manager = LSP_MANAGER.lock().await;
    manager.completion(url, position).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_hover(
    uri: String,
    line: u32,
    character: u32,
) -> Result<Option<Hover>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let position = Position { line, character };

    let manager = LSP_MANAGER.lock().await;
    manager.hover(url, position).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_goto_definition(
    uri: String,
    line: u32,
    character: u32,
) -> Result<Option<GotoDefinitionResponse>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let position = Position { line, character };

    let manager = LSP_MANAGER.lock().await;
    manager.goto_definition(url, position).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_references(
    uri: String,
    line: u32,
    character: u32,
    include_declaration: bool,
) -> Result<Option<Vec<Location>>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let position = Position { line, character };

    let manager = LSP_MANAGER.lock().await;
    manager.references(url, position, include_declaration).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_document_symbols(
    uri: String,
) -> Result<Option<DocumentSymbolResponse>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;

    let manager = LSP_MANAGER.lock().await;
    manager.document_symbols(url).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_code_action(
    uri: String,
    start_line: u32,
    start_char: u32,
    end_line: u32,
    end_char: u32,
    diagnostics: Vec<Diagnostic>,
) -> Result<Option<CodeActionResponse>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let range = Range {
        start: Position { line: start_line, character: start_char },
        end: Position { line: end_line, character: end_char },
    };
    let context = CodeActionContext {
        diagnostics,
        only: None,
        trigger_kind: None,
    };

    let manager = LSP_MANAGER.lock().await;
    manager.code_action(url, range, context).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_rename(
    uri: String,
    line: u32,
    character: u32,
    new_name: String,
) -> Result<Option<WorkspaceEdit>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let position = Position { line, character };

    let manager = LSP_MANAGER.lock().await;
    manager.rename(url, position, new_name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_formatting(
    uri: String,
    tab_size: u32,
    insert_spaces: bool,
) -> Result<Option<Vec<TextEdit>>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let options = FormattingOptions {
        tab_size,
        insert_spaces,
        ..Default::default()
    };

    let manager = LSP_MANAGER.lock().await;
    manager.formatting(url, options).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_signature_help(
    uri: String,
    line: u32,
    character: u32,
) -> Result<Option<SignatureHelp>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let position = Position { line, character };

    let manager = LSP_MANAGER.lock().await;
    manager.signature_help(url, position).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_did_open(
    uri: String,
    language_id: String,
    version: i32,
    text: String,
) -> Result<(), String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;

    let manager = LSP_MANAGER.lock().await;
    manager.did_open(url, language_id, version, text).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_did_change(
    uri: String,
    version: i32,
    changes: Vec<TextDocumentContentChangeEvent>,
) -> Result<(), String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;

    let manager = LSP_MANAGER.lock().await;
    manager.did_change(url, version, changes).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_did_close(
    uri: String,
) -> Result<(), String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;

    let manager = LSP_MANAGER.lock().await;
    manager.did_close(url).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_did_save(
    uri: String,
    text: Option<String>,
) -> Result<(), String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;

    let manager = LSP_MANAGER.lock().await;
    manager.did_save(url, text).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_lsp_configs() -> Result<Vec<LSPConfigInfo>, String> {
    let configs = LSP_CONFIGS.lock().await;
    let server_manager = get_server_manager().await.ok();
    let installed = server_manager.map(|m| m.get_installed_servers()).unwrap_or_default();
    
    let mut result = Vec::new();

    for (lang, config) in configs.iter() {
        let installed_server = installed.get(&config.name);
        result.push(LSPConfigInfo {
            language: lang.clone(),
            name: config.name.clone(),
            command: config.command.clone(),
            file_extensions: config.file_extensions.clone(),
            root_patterns: config.root_patterns.clone(),
            available: installed_server.is_some(),
            installed_version: installed_server.map(|s| s.version.clone()),
            installed_path: installed_server.map(|s| s.path.display().to_string()),
        });
    }

    Ok(result)
}

#[tauri::command]
pub async fn lsp_install_server(language: String) -> Result<String, String> {
    let configs = LSP_CONFIGS.lock().await;
    let config = configs.get(&language).ok_or_else(|| format!("Unknown language: {}", language))?.clone();
    drop(configs);
    
    let server_manager = get_server_manager().await?;
    server_manager.ensure_server(&config).await.map_err(|e| e.to_string())?;
    
    Ok(format!("Installed {}", config.name))
}

#[tauri::command]
pub async fn lsp_uninstall_server(name: String) -> Result<String, String> {
    let server_manager = get_server_manager().await?;
    server_manager.uninstall_server(&name).await.map_err(|e| e.to_string())?;
    Ok(format!("Uninstalled {}", name))
}

#[tauri::command]
pub async fn lsp_check_updates() -> Result<Vec<ServerUpdate>, String> {
    let server_manager = get_server_manager().await?;
    server_manager.check_updates().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_update_server(name: String) -> Result<String, String> {
    let server_manager = get_server_manager().await?;
    server_manager.update_server(&name).await.map_err(|e| e.to_string())?;
    Ok(format!("Updated {}", name))
}

#[tauri::command]
pub async fn lsp_get_installed_servers() -> Result<Vec<InstalledServer>, String> {
    let server_manager = get_server_manager().await?;
    Ok(server_manager.get_installed_servers().into_values().collect())
}

#[tauri::command]
pub async fn lsp_semantic_tokens_full(
    uri: String,
) -> Result<Option<SemanticTokens>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let manager = LSP_MANAGER.lock().await;
    manager.semantic_tokens_full(url).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_semantic_tokens_range(
    uri: String,
    start_line: u32,
    start_char: u32,
    end_line: u32,
    end_char: u32,
) -> Result<Option<SemanticTokens>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let range = Range {
        start: Position { line: start_line, character: start_char },
        end: Position { line: end_line, character: end_char },
    };
    let manager = LSP_MANAGER.lock().await;
    manager.semantic_tokens_range(url, range).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn lsp_semantic_tokens_full_delta(
    uri: String,
    previous_result_id: String,
) -> Result<Option<SemanticTokensFullDeltaResult>, String> {
    let url = Url::parse(&uri).map_err(|e| e.to_string())?;
    let manager = LSP_MANAGER.lock().await;
    manager.semantic_tokens_full_delta(url, previous_result_id).await.map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
pub struct LSPConfigInfo {
    pub language: String,
    pub name: String,
    pub command: Vec<String>,
    pub file_extensions: Vec<String>,
    pub root_patterns: Vec<String>,
    pub available: bool,
    pub installed_version: Option<String>,
    pub installed_path: Option<String>,
}