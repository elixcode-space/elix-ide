use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::sync::Mutex as TokioMutex;
use tracing::{info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapServerConfig {
    pub name: String,
    pub command: Vec<String>,
    pub languages: Vec<String>,
    pub file_extensions: Vec<String>,
    pub download_url: Option<String>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSession {
    pub id: String,
    pub server_name: String,
    pub workspace_root: PathBuf,
    pub program: Option<String>,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,
    pub status: DebugSessionStatus,
    pub process: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebugSessionStatus {
    Starting,
    Running,
    Paused,
    Stopped,
    Terminated,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DapMessage {
    pub seq: u64,
    pub type_: String,
    pub command: Option<String>,
    pub request_seq: Option<u64>,
    pub success: Option<bool>,
    pub message: Option<String>,
    pub body: Option<Value>,
}

impl DapMessage {
    pub fn request(seq: u64, command: &str, arguments: Option<Value>) -> Self {
        Self {
            seq,
            type_: "request".to_string(),
            command: Some(command.to_string()),
            request_seq: None,
            success: None,
            message: None,
            body: arguments,
        }
    }
    
    pub fn response(seq: u64, request_seq: u64, success: bool, body: Option<Value>, message: Option<String>) -> Self {
        Self {
            seq,
            type_: "response".to_string(),
            command: None,
            request_seq: Some(request_seq),
            success: Some(success),
            message,
            body,
        }
    }
    
    pub fn event(seq: u64, event: &str, body: Option<Value>) -> Self {
        Self {
            seq,
            type_: "event".to_string(),
            command: None,
            request_seq: None,
            success: None,
            message: None,
            body: Some(json!({ "event": event, "body": body })),
        }
    }
}

pub struct DapClient {
    server_name: String,
    child: Option<Child>,
    stdin: Option<Arc<TokioMutex<tokio::process::ChildStdin>>>,
    reader: Option<BufReader<tokio::process::ChildStdout>>,
    request_id: TokioMutex<u64>,
    pending_requests: Arc<TokioMutex<HashMap<u64, tokio::sync::oneshot::Sender<DapMessage>>>>,
    event_sender: mpsc::UnboundedSender<DapMessage>,
    workspace_root: PathBuf,
}

impl DapClient {
    pub fn new(server_name: String, workspace_root: PathBuf, event_sender: mpsc::UnboundedSender<DapMessage>) -> Self {
        Self {
            server_name,
            child: None,
            stdin: None,
            reader: None,
            request_id: TokioMutex::new(1),
            pending_requests: Arc::new(TokioMutex::new(HashMap::new())),
            event_sender,
            workspace_root,
        }
    }
    
    pub async fn start(&mut self, command: Vec<String>, args: Vec<String>) -> Result<()> {
        let mut cmd = Command::new(&command[0]);
        cmd.args(&command[1..])
            .args(&args)
            .current_dir(&self.workspace_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        
        let mut child = cmd.spawn()?;
        
        self.stdin = Some(Arc::new(TokioMutex::new(child.stdin.take().unwrap())));
        self.reader = Some(BufReader::new(child.stdout.take().unwrap()));
        self.child = Some(child);
        
        self.start_reader().await;
        
        // Send initialize request
        let init_args = json!({
            "adapterID": self.server_name,
            "linesStartAt1": true,
            "columnsStartAt1": true,
            "supportsVariableType": true,
            "supportsVariablePaging": true,
            "supportsRunInTerminalRequest": true,
            "supportsMemoryReferences": true,
            "supportsProgressReporting": true,
            "supportsInvalidatedEvent": true,
            "supportsMemoryEvent": true,
        });
        
        let response = self.send_request("initialize", Some(init_args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("DAP initialize failed: {:?}", response.message));
        }
        
        // Send initialized event
        self.send_notification("initialized", None).await?;
        
        info!("DAP server {} started", self.server_name);
        Ok(())
    }
    
    async fn start_reader(&mut self) {
        let mut reader = self.reader.take().unwrap();
        let pending = self.pending_requests.clone();
        let event_sender = self.event_sender.clone();
        
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
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
                                if reader.read_line(&mut headers).await.unwrap_or(0) == 0 {
                                    break;
                                }
                                if headers.trim().is_empty() {
                                    break;
                                }
                            }
                            
                            let mut body = vec![0; content_length];
                            if reader.read_exact(&mut body).await.is_ok() {
                                if let Ok(msg) = serde_json::from_slice::<DapMessage>(&body) {
                                    if let Some(id) = msg.request_seq {
                                        let mut pending_guard = pending.lock().await;
                                        if let Some(sender) = pending_guard.remove(&id) {
                                            let _ = sender.send(msg);
                                        }
                                    } else {
                                        // It's an event
                                        let _ = event_sender.send(msg);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Error reading from DAP server: {}", e);
                        break;
                    }
                }
            }
        });
    }
    
    async fn send_request(&self, command: &str, arguments: Option<Value>) -> Result<DapMessage> {
        let id = {
            let mut id = self.request_id.lock().await;
            *id += 1;
            *id
        };
        
        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending_requests.lock().await.insert(id, tx);
        
        let request = DapMessage::request(id, command, arguments);
        let json = serde_json::to_vec(&request)?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());
        
        if let Some(stdin) = &self.stdin {
            let mut stdin_guard = stdin.lock().await;
            stdin_guard.write_all(header.as_bytes()).await?;
            stdin_guard.write_all(&json).await?;
            stdin_guard.flush().await?;
        }
        
        match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(anyhow::anyhow!("Request channel closed")),
            Err(_) => Err(anyhow::anyhow!("Request timeout")),
        }
    }
    
    async fn send_notification(&self, event: &str, body: Option<Value>) -> Result<()> {
        let id = {
            let mut id = self.request_id.lock().await;
            *id += 1;
            *id
        };
        
        let notification = DapMessage::event(id, event, body);
        let json = serde_json::to_vec(&notification)?;
        let header = format!("Content-Length: {}\r\n\r\n", json.len());
        
        if let Some(stdin) = &self.stdin {
            let mut stdin_guard = stdin.lock().await;
            stdin_guard.write_all(header.as_bytes()).await?;
            stdin_guard.write_all(&json).await?;
            stdin_guard.flush().await?;
        }
        
        Ok(())
    }
    
    pub async fn launch(&self, program: &str, args: Vec<String>, cwd: PathBuf, env: HashMap<String, String>) -> Result<()> {
        let args_json = json!({
            "program": program,
            "args": args,
            "cwd": cwd.to_string_lossy().to_string(),
            "env": env,
            "stopOnEntry": false,
            "console": "internalConsole",
        });
        
        let response = self.send_request("launch", Some(args_json)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Launch failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn attach(&self, process_id: u32) -> Result<()> {
        let args = json!({ "processId": process_id });
        let response = self.send_request("attach", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Attach failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn set_breakpoints(&self, source_path: &str, breakpoints: Vec<SourceBreakpoint>) -> Result<Vec<Breakpoint>> {
        let args = json!({
            "source": { "path": source_path },
            "breakpoints": breakpoints,
            "sourceModified": false,
        });
        
        let response = self.send_request("setBreakpoints", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Set breakpoints failed: {:?}", response.message));
        }
        
        let breakpoints: Vec<Breakpoint> = serde_json::from_value(
            response.body.unwrap_or(json!({}))["breakpoints"].clone()
        ).unwrap_or_default();
        
        Ok(breakpoints)
    }
    
    pub async fn continue_execution(&self, thread_id: u64) -> Result<()> {
        let args = json!({ "threadId": thread_id });
        let response = self.send_request("continue", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Continue failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn pause(&self, thread_id: u64) -> Result<()> {
        let args = json!({ "threadId": thread_id });
        let response = self.send_request("pause", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Pause failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn step_over(&self, thread_id: u64) -> Result<()> {
        let args = json!({ "threadId": thread_id });
        let response = self.send_request("next", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Step over failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn step_into(&self, thread_id: u64) -> Result<()> {
        let args = json!({ "threadId": thread_id });
        let response = self.send_request("stepIn", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Step into failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn step_out(&self, thread_id: u64) -> Result<()> {
        let args = json!({ "threadId": thread_id });
        let response = self.send_request("stepOut", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Step out failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn get_threads(&self) -> Result<Vec<Thread>> {
        let response = self.send_request("threads", None).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Get threads failed: {:?}", response.message));
        }
        
        let threads: Vec<Thread> = serde_json::from_value(
            response.body.unwrap_or(json!({}))["threads"].clone()
        ).unwrap_or_default();
        
        Ok(threads)
    }
    
    pub async fn get_stack_trace(&self, thread_id: u64, start_frame: u32, levels: u32) -> Result<Vec<StackFrame>> {
        let args = json!({
            "threadId": thread_id,
            "startFrame": start_frame,
            "levels": levels,
        });
        
        let response = self.send_request("stackTrace", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Get stack trace failed: {:?}", response.message));
        }
        
        let frames: Vec<StackFrame> = serde_json::from_value(
            response.body.unwrap_or(json!({}))["stackFrames"].clone()
        ).unwrap_or_default();
        
        Ok(frames)
    }
    
    pub async fn get_scopes(&self, frame_id: u64) -> Result<Vec<Scope>> {
        let args = json!({ "frameId": frame_id });
        let response = self.send_request("scopes", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Get scopes failed: {:?}", response.message));
        }
        
        let scopes: Vec<Scope> = serde_json::from_value(
            response.body.unwrap_or(json!({}))["scopes"].clone()
        ).unwrap_or_default();
        
        Ok(scopes)
    }
    
    pub async fn get_variables(&self, variables_reference: u64) -> Result<Vec<Variable>> {
        let args = json!({ "variablesReference": variables_reference });
        let response = self.send_request("variables", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Get variables failed: {:?}", response.message));
        }
        
        let variables: Vec<Variable> = serde_json::from_value(
            response.body.unwrap_or(json!({}))["variables"].clone()
        ).unwrap_or_default();
        
        Ok(variables)
    }
    
    pub async fn evaluate(&self, expression: &str, frame_id: Option<u64>, context: &str) -> Result<EvaluateResult> {
        let args = json!({
            "expression": expression,
            "frameId": frame_id,
            "context": context,
        });
        
        let response = self.send_request("evaluate", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Evaluate failed: {:?}", response.message));
        }
        
        let result: EvaluateResult = serde_json::from_value(
            response.body.unwrap_or(json!({}))
        ).unwrap_or_default();
        
        Ok(result)
    }
    
    pub async fn disconnect(&self, terminate_debuggee: bool) -> Result<()> {
        let args = json!({ "terminateDebuggee": terminate_debuggee });
        let response = self.send_request("disconnect", Some(args)).await?;
        if !response.success.unwrap_or(false) {
            return Err(anyhow::anyhow!("Disconnect failed: {:?}", response.message));
        }
        Ok(())
    }
    
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        Ok(())
    }
}

pub struct DapManager {
    clients: Arc<TokioMutex<HashMap<String, Arc<TokioMutex<DapClient>>>>>,
    event_sender: mpsc::UnboundedSender<DapMessage>,
}

impl Clone for DapManager {
    fn clone(&self) -> Self {
        Self {
            clients: self.clients.clone(),
            event_sender: self.event_sender.clone(),
        }
    }
}

impl DapManager {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<DapMessage>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let manager = Self {
            clients: Arc::new(TokioMutex::new(HashMap::new())),
            event_sender: tx,
        };
        (manager, rx)
    }
    
    pub async fn get_or_create_client(
        &self,
        workspace_root: PathBuf,
        server_name: &str,
        command: Vec<String>,
        args: Vec<String>,
    ) -> Result<Arc<TokioMutex<DapClient>>> {
        let key = format!("{}::{}", workspace_root.display(), server_name);
        
        {
            let clients = self.clients.lock().await;
            if let Some(client) = clients.get(&key) {
                return Ok(client.clone());
            }
        }
        
        let client = Arc::new(TokioMutex::new(DapClient::new(
            server_name.to_string(),
            workspace_root,
            self.event_sender.clone(),
        )));
        
        client.lock().await.start(command, args).await?;
        
        let mut clients = self.clients.lock().await;
        clients.insert(key, client.clone());
        Ok(client)
    }
    
    pub async fn shutdown_all(&self) -> Result<()> {
        let mut clients = self.clients.lock().await;
        for (_, client) in clients.drain() {
            client.lock().await.shutdown().await?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceBreakpoint {
    pub line: u32,
    pub column: Option<u32>,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breakpoint {
    pub id: u64,
    pub verified: bool,
    pub line: u32,
    pub column: Option<u32>,
    pub message: Option<String>,
    pub source: Option<Source>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: Option<String>,
    pub path: Option<String>,
    pub source_reference: u64,
    pub presentation_hint: Option<String>,
    pub origin: Option<String>,
    pub sources: Option<Vec<Source>>,
    pub adapter_data: Option<Value>,
    pub checksums: Option<Vec<Checksum>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksum {
    pub algorithm: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub id: u64,
    pub name: String,
    pub source: Option<Source>,
    pub line: u32,
    pub column: u32,
    pub end_line: Option<u32>,
    pub end_column: Option<u32>,
    pub module_id: Option<Value>,
    pub presentation_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub name: String,
    pub presentation_hint: Option<String>,
    pub variables_reference: u64,
    pub named_variables: Option<u32>,
    pub indexed_variables: Option<u32>,
    pub expensive: bool,
    pub source: Option<Source>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub type_: Option<String>,
    pub presentation_hint: Option<VariablePresentationHint>,
    pub evaluate_name: Option<String>,
    pub variables_reference: u64,
    pub named_variables: Option<u32>,
    pub indexed_variables: Option<u32>,
    pub memory_reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariablePresentationHint {
    pub kind: Option<String>,
    pub attributes: Option<Vec<String>>,
    pub visibility: Option<String>,
    pub lazy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateResult {
    pub result: String,
    pub type_: Option<String>,
    pub presentation_hint: Option<VariablePresentationHint>,
    pub variables_reference: u64,
    pub named_variables: Option<u32>,
    pub indexed_variables: Option<u32>,
    pub memory_reference: Option<String>,
}

impl Default for EvaluateResult {
    fn default() -> Self {
        Self {
            result: String::new(),
            type_: None,
            presentation_hint: None,
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
        }
    }
}

impl Default for DapManager {
    fn default() -> Self {
        let (manager, _rx) = Self::new();
        manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_dap_manager_creation() {
        let (manager, _rx) = DapManager::new();
        assert!(manager.clients.lock().await.is_empty());
    }
}