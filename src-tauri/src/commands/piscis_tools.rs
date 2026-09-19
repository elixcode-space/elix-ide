//! Piscis Engine Tools Integration for Tauri
//!
//! Exposes piscis-engine's native agent tools (file operations, shell, web, etc.)
//! as Tauri commands for agentic IDE capabilities.

use crate::core_engine::{create_tool_registry, get_kernel_state};
use piscis_kernel::agent::tool::{ToolRegistry, ToolContext, ToolSettings};
use piscis_kernel::store::{db::Database, settings::Settings};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{AppHandle, Emitter, State, Window};
use uuid::Uuid;

// Tool definition for frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Tool execution request
#[derive(Debug, Clone, Deserialize)]
pub struct ExecuteToolRequest {
    pub tool_name: String,
    pub parameters: Value,
    pub session_id: Option<String>,
    pub workspace_root: Option<String>,
}

/// Tool execution response
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteToolResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub tool_use_id: Option<String>,
}

/// List available tools
#[tauri::command]
pub async fn list_piscis_tools() -> Result<Vec<ToolDefinition>, String> {
    // Return a static list of available piscis tools
    // In production, this would be dynamically generated from the registry
    Ok(vec![
        ToolDefinition {
            name: "file_read".to_string(),
            description: "Read a file from the workspace".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path relative to workspace" }
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "file_write".to_string(),
            description: "Write a file to the workspace".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "File path relative to workspace" },
                    "content": { "type": "string", "description": "File content" }
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "file_edit".to_string(),
            description: "Edit a file using search/replace".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_text": { "type": "string" },
                    "new_text": { "type": "string" }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        },
        ToolDefinition {
            name: "file_list".to_string(),
            description: "List files in a directory".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path" }
                }
            }),
        },
        ToolDefinition {
            name: "file_search".to_string(),
            description: "Search for text in files".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "shell".to_string(),
            description: "Execute a shell command".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "cwd": { "type": "string" },
                    "timeout_ms": { "type": "number" }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "code_run".to_string(),
            description: "Run code in a language-specific environment".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "language": { "type": "string" },
                    "code": { "type": "string" },
                    "files": { "type": "object" }
                },
                "required": ["language", "code"]
            }),
        },
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "max_results": { "type": "number" }
                },
                "required": ["query"]
            }),
        },
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch content from a URL".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" },
                    "max_length": { "type": "number" }
                },
                "required": ["url"]
            }),
        },
        ToolDefinition {
            name: "plan_todo".to_string(),
            description: "Create a todo list for task planning".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "items": { 
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["items"]
            }),
        },
        ToolDefinition {
            name: "memory_store".to_string(),
            description: "Store a memory for later recall".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string" },
                    "value": { "type": "string" }
                },
                "required": ["key", "value"]
            }),
        },
        ToolDefinition {
            name: "memory_recall".to_string(),
            description: "Recall a stored memory".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "key": { "type": "string" }
                },
                "required": ["key"]
            }),
        },
    ])
}

/// Execute a piscis tool
#[tauri::command]
pub async fn execute_piscis_tool(
    request: ExecuteToolRequest,
    window: Window,
) -> Result<ExecuteToolResponse, String> {
    let session_id = request.session_id.unwrap_or_else(|| format!("session-{}", Uuid::new_v4().simple()));
    let workspace_root = request.workspace_root.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    // Get or create tool context
    let tool_context = {
        let mut contexts = PISCIS_TOOL_CONTEXTS.lock().await;
        contexts.entry(session_id.clone()).or_insert_with(|| {
            ToolContext {
                session_id: session_id.clone(),
                workspace_root: PathBuf::from(&workspace_root),
                bypass_permissions: true,
                settings: Arc::new(ToolSettings::default()),
                max_iterations: Some(50),
                memory_owner_id: "piscis".to_string(),
                pool_session_id: None,
                tool_use_id: Some(Uuid::new_v4().to_string()),
                cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                loop_halt: None,
            }
        }).clone()
    };

    let tool_use_id = tool_context.tool_use_id.clone();
    
    // Emit tool use start event
    let _ = window.emit("piscis-tool-start", serde_json::json!({
        "session_id": session_id,
        "tool": request.tool_name,
        "tool_use_id": tool_use_id,
        "parameters": request.parameters
    }));

    // Get tool registry from core engine
    let kernel_state = get_kernel_state().await
        .ok_or("Core engine not initialized. Call initialize_core first.")?;
    
    let (db, settings) = &kernel_state;
    let tool_registry = create_tool_registry(db, settings);

    // Find and execute tool
    let tool = tool_registry.get(&request.tool_name)
        .ok_or_else(|| format!("Tool '{}' not found", request.tool_name))?;

    // Create tool input
    let tool_input = request.parameters;

    // Execute tool
    match tool.call(tool_input, &tool_context).await {
        Ok(result) => {
            let _ = window.emit("piscis-tool-complete", serde_json::json!({
                "session_id": session_id,
                "tool": request.tool_name,
                "tool_use_id": tool_use_id,
                "result": result.content,
                "is_error": result.is_error
            }));
            
            Ok(ExecuteToolResponse {
                success: !result.is_error,
                result: Some(serde_json::json!(result.content)),
                error: if result.is_error { Some(result.content) } else { None },
                tool_use_id,
            })
        }
        Err(e) => {
            let _ = window.emit("piscis-tool-error", serde_json::json!({
                "session_id": session_id,
                "tool": request.tool_name,
                "tool_use_id": tool_use_id,
                "error": e.to_string()
            }));
            
            Ok(ExecuteToolResponse {
                success: false,
                result: None,
                error: Some(e.to_string()),
                tool_use_id,
            })
        }
    }
}

/// Initialize piscis tools with kernel state
#[tauri::command]
pub async fn initialize_piscis_tools(app: AppHandle) -> Result<(), String> {
    let kernel_state = get_kernel_state().await
        .ok_or("Core engine not initialized")?;
    
    let (db, settings) = &kernel_state;
    let tool_registry = create_tool_registry(db, settings);
    
    let mut registry_guard = PISCIS_TOOL_REGISTRY.lock().await;
    *registry_guard = Some(tool_registry);
    
    Ok(())
}

// Global state for tool contexts
static PISCIS_TOOL_CONTEXTS: once_cell::sync::Lazy<Mutex<HashMap<String, ToolContext>>> = 
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

static PISCIS_TOOL_REGISTRY: once_cell::sync::Lazy<Mutex<Option<ToolRegistry>>> = 
    once_cell::sync::Lazy::new(|| Mutex::new(None));