use lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tower_lsp::lsp_types::request::*;
use tokio::sync::{Mutex, mpsc, RwLock, broadcast};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use anyhow::Result;
use tracing::{info, warn, error};

pub struct LSPClient {
    client: Option<Client>,
    server_process: Option<tokio::process::Child>,
    stdin: Option<Arc<Mutex<tokio::process::ChildStdin>>>,
    stdout: Option<BufReader<tokio::process::ChildStdout>>,
    request_id: Mutex<u64>,
    pending_requests: Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>>,
    capabilities: Mutex<Option<ServerCapabilities>>,
    initialized: Mutex<bool>,
    workspace_root: PathBuf,
    server_name: String,
    notification_tx: broadcast::Sender<serde_json::Value>,
}

impl LSPClient {
    pub fn new(workspace_root: PathBuf, server_name: String) -> Self {
        let (notification_tx, _) = broadcast::channel(100);
        Self {
            client: None,
            server_process: None,
            stdin: None,
            stdout: None,
            request_id: Mutex::new(1),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            capabilities: Mutex::new(None),
            initialized: Mutex::new(false),
            workspace_root,
            server_name,
            notification_tx,
        }
    }
    
    pub async fn start(&mut self, command: Vec<String>) -> Result<()> {
        let mut cmd = Command::new(&command[0]);
        cmd.args(&command[1..])
            .current_dir(&self.workspace_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        
        let mut child = cmd.spawn()?;
        
        self.stdin = Some(Arc::new(Mutex::new(child.stdin.take().unwrap())));
        self.stdout = Some(BufReader::new(child.stdout.take().unwrap()));
        self.server_process = Some(child);
        
        // Start reading responses
        self.start_response_reader().await;
        
        // Send initialize
        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(Url::from_file_path(&self.workspace_root).unwrap()),
            root_path: Some(self.workspace_root.to_string_lossy().to_string()),
            capabilities: ClientCapabilities::default(),
            initialization_options: None,
            trace: Some(TraceValue::Off),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(&self.workspace_root).unwrap(),
                name: self.workspace_root.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            }]),
            client_info: Some(ClientInfo {
                name: "ElixirIDE".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            locale: None,
        };
        
        let result = self.send_request("initialize", init_params).await?;
        let init_result: InitializeResult = serde_json::from_value(result)?;
        
        *self.capabilities.lock().await = Some(init_result.capabilities);
        *self.initialized.lock().await = true;
        
        // Send initialized notification
        self.send_notification("initialized", InitializedParams {}).await?;
        
        info!("LSP server {} started", self.server_name);
        Ok(())
    }
    
    async fn start_response_reader(&mut self) {
        let mut stdout = self.stdout.take().unwrap();
        let pending = self.pending_requests.clone();
        let notification_tx = self.notification_tx.clone();
        
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match stdout.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        if line.trim().is_empty() {
                            continue;
                        }
                        
                        // Parse Content-Length header
                        if line.starts_with("Content-Length:") {
                            let content_length: usize = line[15..].trim().parse().unwrap_or(0);
                            let mut headers = String::new();
                            loop {
                                headers.clear();
                                if stdout.read_line(&mut headers).await.unwrap_or(0) == 0 {
                                    break;
                                }
                                if headers.trim().is_empty() {
                                    break;
                                }
                            }
                            
                            let mut body = vec![0; content_length];
                            use tokio::io::AsyncReadExt;
                            if stdout.read_exact(&mut body).await.is_ok() {
                                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body) {
                                    if let Some(id) = json.get("id").and_then(|v| v.as_u64()) {
                                        // It's a response to a request
                                        if let Some(sender) = pending.lock().await.remove(&id) {
                                            let _ = sender.send(json);
                                        }
                                    } else {
                                        // It's a notification (no id field)
                                        let _ = notification_tx.send(json);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading from LSP server: {}", e);
                        break;
                    }
                }
            }
        });
    }
    
    async fn send_request<P: serde::Serialize>(&self, method: &str, params: P) -> Result<serde_json::Value> {
        let id = {
            let mut id = self.request_id.lock().await;
            *id += 1;
            *id
        };
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_requests.lock().await.insert(id, tx);
        
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        
        self.write_message(request).await?;
        
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => {
                if let Some(error) = response.get("error") {
                    return Err(anyhow::anyhow!("LSP error: {}", error));
                }
                Ok(response.get("result").cloned().unwrap_or(serde_json::Value::Null))
            }
            Ok(Err(_)) => Err(anyhow::anyhow!("Request channel closed")),
            Err(_) => Err(anyhow::anyhow!("Request timeout")),
        }
    }
    
    async fn send_notification<P: serde::Serialize>(&self, method: &str, params: P) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        
        self.write_message(notification).await
    }
    
    async fn write_message(&self, message: serde_json::Value) -> Result<()> {
        let body = serde_json::to_vec(&message)?;
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        
        if let Some(stdin) = &self.stdin {
            let mut stdin = stdin.lock().await;
            stdin.write_all(header.as_bytes()).await?;
            stdin.write_all(&body).await?;
            stdin.flush().await?;
        }
        Ok(())
    }
    
    pub async fn shutdown(&mut self) -> Result<()> {
        if *self.initialized.lock().await {
            self.send_request("shutdown", ()).await?;
            self.send_notification("exit", ()).await?;
        }
        
        if let Some(mut process) = self.server_process.take() {
            let _ = process.kill().await;
        }
        Ok(())
    }
    
    // LSP Methods
    pub async fn did_open(&self, uri: Url, language_id: String, version: i32, text: String) -> Result<()> {
        self.send_notification("textDocument/didOpen", DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id,
                version,
                text,
            },
        }).await
    }
    
    pub async fn did_change(&self, uri: Url, version: i32, changes: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        self.send_notification("textDocument/didChange", DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: changes,
        }).await
    }
    
    pub async fn did_close(&self, uri: Url) -> Result<()> {
        self.send_notification("textDocument/didClose", DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        }).await
    }
    
    pub async fn did_save(&self, uri: Url, text: Option<String>) -> Result<()> {
        self.send_notification("textDocument/didSave", DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text,
        }).await
    }
    
    pub fn subscribe_diagnostics(&self) -> broadcast::Receiver<serde_json::Value> {
        self.notification_tx.subscribe()
    }
    
    pub async fn semantic_tokens_full(&self, uri: Url) -> Result<Option<SemanticTokens>> {
        let result = self.send_request("textDocument/semanticTokens/full", SemanticTokensParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn semantic_tokens_range(&self, uri: Url, range: Range) -> Result<Option<SemanticTokens>> {
        let result = self.send_request("textDocument/semanticTokens/range", SemanticTokensRangeParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn semantic_tokens_full_delta(&self, uri: Url, previous_result_id: String) -> Result<Option<SemanticTokensFullDeltaResult>> {
        let result = self.send_request("textDocument/semanticTokens/full/delta", SemanticTokensDeltaParams {
            text_document: TextDocumentIdentifier { uri },
            previous_result_id,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn completion(&self, uri: Url, position: Position, context: Option<CompletionContext>) -> Result<Option<CompletionResponse>> {
        let result = self.send_request("textDocument/completion", CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context,
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn hover(&self, uri: Url, position: Position) -> Result<Option<Hover>> {
        let result = self.send_request("textDocument/hover", HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn goto_definition(&self, uri: Url, position: Position) -> Result<Option<GotoDefinitionResponse>> {
        let result = self.send_request("textDocument/definition", GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn references(&self, uri: Url, position: Position, include_declaration: bool) -> Result<Option<Vec<Location>>> {
        let result = self.send_request("textDocument/references", ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: ReferenceContext { include_declaration },
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn document_symbols(&self, uri: Url) -> Result<Option<DocumentSymbolResponse>> {
        let result = self.send_request("textDocument/documentSymbol", DocumentSymbolParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn code_action(&self, uri: Url, range: Range, context: CodeActionContext) -> Result<Option<CodeActionResponse>> {
        let result = self.send_request("textDocument/codeAction", CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            context,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn rename(&self, uri: Url, position: Position, new_name: String) -> Result<Option<WorkspaceEdit>> {
        let result = self.send_request("textDocument/rename", RenameParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            new_name,
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn formatting(&self, uri: Url, options: FormattingOptions) -> Result<Option<Vec<TextEdit>>> {
        let result = self.send_request("textDocument/formatting", DocumentFormattingParams {
            text_document: TextDocumentIdentifier { uri },
            options,
            work_done_progress_params: Default::default(),
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
    
    pub async fn signature_help(&self, uri: Url, position: Position) -> Result<Option<SignatureHelp>> {
        let result = self.send_request("textDocument/signatureHelp", SignatureHelpParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: Default::default(),
            context: None,
        }).await?;
        
        Ok(serde_json::from_value(result).ok())
    }
}

pub struct LSPManager {
    clients: RwLock<HashMap<String, Arc<Mutex<LSPClient>>>>,
}

impl LSPManager {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
        }
    }
    
    pub async fn get_or_create_client(
        &self,
        workspace_root: PathBuf,
        server_name: &str,
        command: Vec<String>,
    ) -> Result<Arc<Mutex<LSPClient>>> {
        let key = format!("{}::{}", workspace_root.display(), server_name);
        
        {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&key) {
                return Ok(client.clone());
            }
        }
        
        let client = Arc::new(Mutex::new(LSPClient::new(workspace_root, server_name.to_string())));
        client.lock().await.start(command).await?;
        
        let mut clients = self.clients.write().await;
        clients.insert(key, client.clone());
        Ok(client)
    }
    
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut clients = self.clients.write().await;
        for (_, client) in clients.drain() {
            client.lock().await.shutdown().await?;
        }
        Ok(())
    }
    
    fn find_client_for_uri(&self, uri: &Url) -> Option<Arc<Mutex<LSPClient>>> {
        // Find client whose workspace root matches the URI
        let clients = self.clients.try_read().ok()?;
        for (key, client) in clients.iter() {
            // Extract workspace root from key
            if let Some(workspace_str) = key.split("::").next() {
                let workspace_root = PathBuf::from(workspace_str);
                if let Ok(file_path) = uri.to_file_path() {
                    if file_path.starts_with(&workspace_root) {
                        return Some(client.clone());
                    }
                }
            }
        }
        // Fallback: return first client
        clients.values().next().cloned()
    }
    
    pub async fn completion(&self, uri: Url, position: Position) -> Result<Option<CompletionResponse>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.completion(uri, position, None).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn hover(&self, uri: Url, position: Position) -> Result<Option<Hover>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.hover(uri, position).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn goto_definition(&self, uri: Url, position: Position) -> Result<Option<GotoDefinitionResponse>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.goto_definition(uri, position).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn references(&self, uri: Url, position: Position, include_declaration: bool) -> Result<Option<Vec<Location>>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.references(uri, position, include_declaration).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn document_symbols(&self, uri: Url) -> Result<Option<DocumentSymbolResponse>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.document_symbols(uri).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn code_action(&self, uri: Url, range: Range, context: CodeActionContext) -> Result<Option<CodeActionResponse>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.code_action(uri, range, context).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn rename(&self, uri: Url, position: Position, new_name: String) -> Result<Option<WorkspaceEdit>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.rename(uri, position, new_name).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn formatting(&self, uri: Url, options: FormattingOptions) -> Result<Option<Vec<TextEdit>>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.formatting(uri, options).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn signature_help(&self, uri: Url, position: Position) -> Result<Option<SignatureHelp>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.signature_help(uri, position).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn did_open(&self, uri: Url, language_id: String, version: i32, text: String) -> Result<()> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.did_open(uri, language_id, version, text).await
        } else {
            Ok(())
        }
    }
    
    pub async fn did_change(&self, uri: Url, version: i32, changes: Vec<TextDocumentContentChangeEvent>) -> Result<()> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.did_change(uri, version, changes).await
        } else {
            Ok(())
        }
    }
    
    pub async fn did_close(&self, uri: Url) -> Result<()> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.did_close(uri).await
        } else {
            Ok(())
        }
    }
    
    pub async fn did_save(&self, uri: Url, text: Option<String>) -> Result<()> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.did_save(uri, text).await
        } else {
            Ok(())
        }
    }
    
    pub async fn semantic_tokens_full(&self, uri: Url) -> Result<Option<SemanticTokens>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.semantic_tokens_full(uri).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn semantic_tokens_range(&self, uri: Url, range: Range) -> Result<Option<SemanticTokens>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.semantic_tokens_range(uri, range).await
        } else {
            Ok(None)
        }
    }
    
    pub async fn semantic_tokens_full_delta(&self, uri: Url, previous_result_id: String) -> Result<Option<SemanticTokensFullDeltaResult>> {
        if let Some(client) = self.find_client_for_uri(&uri) {
            client.lock().await.semantic_tokens_full_delta(uri, previous_result_id).await
        } else {
            Ok(None)
        }
    }
    
    pub fn subscribe_diagnostics(&self) -> broadcast::Receiver<serde_json::Value> {
        // This is a bit tricky - we need to get a client to subscribe
        // For now, we'll need to handle this differently
        // Let's just create a dummy receiver for now
        let (tx, rx) = broadcast::channel(100);
        rx
    }
}

#[derive(Clone)]
pub struct LSPServerConfig {
    pub name: String,
    pub command: Vec<String>,
    pub languages: Vec<String>,
    pub file_extensions: Vec<String>,
    pub root_patterns: Vec<String>,
    pub download_url: Option<String>,
}