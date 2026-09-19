use crate::services::tantivy_index::{CodeIndex, SearchResult};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use once_cell::sync::Lazy;
use tracing::{info, warn, error};

pub struct SearchState {
    index: Mutex<Option<Arc<CodeIndex>>>,
}

static SEARCH_STATE: Lazy<Arc<SearchState>> = Lazy::new(|| Arc::new(SearchState {
    index: Mutex::new(None),
}));

#[tauri::command]
pub async fn initialize_search(app: AppHandle, workspace_root: String) -> Result<String, String> {
    let workspace = PathBuf::from(&workspace_root);
    let index_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("elixide")
        .join("search_index");
    
    let index = CodeIndex::new(workspace.clone(), index_path)
        .map_err(|e| format!("Failed to create search index: {}", e))?;
    
    // Initial indexing in background
    let index_arc = Arc::new(index);
    let index_clone = index_arc.clone();
    let exclude_patterns = vec![
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/.git/**".to_string(),
        "**/dist/**".to_string(),
        "**/build/**".to_string(),
    ];
    
    tokio::spawn(async move {
        if let Err(e) = index_clone.index_workspace(&exclude_patterns) {
            tracing::error!("Initial indexing failed: {}", e);
        }
        
        if let Err(e) = index_clone.watch_workspace(exclude_patterns) {
            tracing::error!("File watcher failed: {}", e);
        }
    });
    
    *SEARCH_STATE.index.lock().unwrap() = Some(index_arc);
    
    Ok("Search initialized".to_string())
}

#[tauri::command]
pub async fn search_code(
    query: String,
    limit: Option<usize>,
    language: Option<String>,
    file_pattern: Option<String>,
) -> Result<Vec<SearchResult>, String> {
    let state = SEARCH_STATE.index.lock().unwrap();
    let index = state.as_ref().ok_or("Search not initialized")?;
    
    let results = index.search(
        &query,
        limit.unwrap_or(100),
        language.as_deref(),
        file_pattern.as_deref(),
    ).map_err(|e| e.to_string())?;
    
    Ok(results)
}

#[tauri::command]
pub async fn search_symbols(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let state = SEARCH_STATE.index.lock().unwrap();
    let index = state.as_ref().ok_or("Search not initialized")?;
    
    let results = index.search_symbols(&query, limit.unwrap_or(100))
        .map_err(|e| e.to_string())?;
    
    Ok(results)
}

#[tauri::command]
pub async fn search_replace(
    query: String,
    replacement: String,
    file_pattern: Option<String>,
    dry_run: bool,
) -> Result<ReplaceResult, String> {
    let state = SEARCH_STATE.index.lock().unwrap();
    let index = state.as_ref().ok_or("Search not initialized")?;
    
    let results = index.search(&query, 1000, None, file_pattern.as_deref())
        .map_err(|e| e.to_string())?;
    
    let mut replaced = 0;
    let mut files_changed = Vec::new();
    
    if !dry_run {
        for result in &results {
            let full_path = index.workspace_root().join(&result.path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                let new_content = content.replace(&query, &replacement);
                if new_content != content {
                    std::fs::write(&full_path, new_content).map_err(|e| e.to_string())?;
                    replaced += 1;
                    files_changed.push(result.path.clone());
                }
            }
        }
        
        // Re-index changed files
        for path in &files_changed {
            let full_path = index.workspace_root().join(path);
            if let Err(e) = index.index_file(&full_path) {
                tracing::warn!("Failed to re-index {}: {}", path, e);
            }
        }
        index.commit().map_err(|e| e.to_string())?;
    }
    
    Ok(ReplaceResult {
        total_matches: results.len(),
        replaced,
        files_changed,
        dry_run,
    })
}

#[tauri::command]
pub async fn get_search_stats() -> Result<SearchStats, String> {
    let state = SEARCH_STATE.index.lock().unwrap();
    let index = state.as_ref().ok_or("Search not initialized")?;
    
    let stats = index.stats().map_err(|e| e.to_string())?;
    
    Ok(SearchStats {
        num_documents: stats.num_docs,
        num_terms: stats.num_terms,
        index_size_bytes: stats.index_size,
    })
}

#[tauri::command]
pub async fn reindex_workspace() -> Result<String, String> {
    let state = SEARCH_STATE.index.lock().unwrap();
    let index = state.as_ref().ok_or("Search not initialized")?;
    
    let exclude_patterns = vec![
        "**/node_modules/**".to_string(),
        "**/target/**".to_string(),
        "**/.git/**".to_string(),
        "**/dist/**".to_string(),
        "**/build/**".to_string(),
    ];
    
    let count = index.index_workspace(&exclude_patterns)
        .map_err(|e| e.to_string())?;
    
    Ok(format!("Re-indexed {} files", count))
}

#[derive(serde::Serialize)]
pub struct ReplaceResult {
    pub total_matches: usize,
    pub replaced: usize,
    pub files_changed: Vec<String>,
    pub dry_run: bool,
}

#[derive(serde::Serialize)]
pub struct SearchStats {
    pub num_documents: u64,
    pub num_terms: u64,
    pub index_size_bytes: u64,
}