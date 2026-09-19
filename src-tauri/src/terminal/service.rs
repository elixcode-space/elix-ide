use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock, mpsc};
use uuid::Uuid;

use portable_pty::{CommandBuilder, PtyPair, PtySize, native_pty_system};
use anyhow::Error as AnyhowError;
use crate::terminal::types::*;

#[derive(thiserror::Error, Debug)]
pub enum TerminalError {
    #[error("PTY error: {0}")]
    Pty(#[from] AnyhowError),
    #[error("Terminal not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type Result<T> = std::result::Result<T, TerminalError>;

struct TerminalSession {
    id: String,
    pty_pair: PtyPair,
    writer: Option<Box<dyn std::io::Write + Send>>,
    reader_task: Option<tokio::task::JoinHandle<()>>,
    cwd: String,
    shell: String,
    cols: u16,
    rows: u16,
    title: String,
    pid: Option<u32>,
    is_active: bool,
    created_at: String,
    last_activity: String,
    event_sender: mpsc::UnboundedSender<(String, TerminalEvent)>,
}

#[derive(Debug, Clone)]
pub enum TerminalEvent {
    Data(String),
    Resize(u16, u16),
    Exit(i32),
    TitleChanged(String),
    CwdChanged(String),
    ProcessStarted(u32, String),
    ProcessExited(u32, i32),
    Bell,
    LinkDetected(TerminalLink),
}

pub struct TerminalService {
    sessions: Arc<RwLock<HashMap<String, Arc<Mutex<TerminalSession>>>>>,
    profiles: Arc<RwLock<Vec<TerminalProfile>>>,
    config: Arc<RwLock<TerminalConfig>>,
    event_sender: mpsc::UnboundedSender<(String, TerminalEvent)>,
    pty_system: Arc<Mutex<Box<dyn portable_pty::PtySystem + Send>>>,
}

impl TerminalService {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<(String, TerminalEvent)>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let pty_system = Arc::new(Mutex::new(native_pty_system()));
        
        let default_config = TerminalConfig {
            profiles: vec![
                TerminalProfile {
                    name: "Default".to_string(),
                    shell: Self::default_shell(),
                    args: vec![],
                    env: HashMap::new(),
                    cwd: None,
                    icon: None,
                    color_scheme: None,
                }
            ],
            default_profile: "Default".to_string(),
            font: TerminalFontOptions {
                family: "Monospace".to_string(),
                size: 14.0,
                weight: "normal".to_string(),
                line_height: 1.2,
                letter_spacing: 0.0,
            },
            color_scheme: Self::default_color_scheme(),
            scrollback: TerminalScrollbackOptions { lines: 10000 },
            cursor: TerminalCursorOptions {
                style: CursorStyle::Block,
                blink: true,
            },
            bell: TerminalBellOptions {
                sound: true,
                visual: false,
            },
            confirm_close: true,
            close_on_exit: false,
            word_separators: " \t\n\r\"'`@$*+=-/\\()[]{}|;:,.<>?!".to_string(),
            right_click_behavior: RightClickBehavior::ShowContextMenu,
        };

        let service = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            profiles: Arc::new(RwLock::new(default_config.profiles.clone())),
            config: Arc::new(RwLock::new(default_config)),
            event_sender: tx,
            pty_system,
        };

        (service, rx)
    }

    fn default_shell() -> String {
        if cfg!(target_os = "windows") {
            "powershell.exe".to_string()
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string())
        }
    }

    fn default_color_scheme() -> TerminalColorScheme {
        TerminalColorScheme {
            name: "Default Dark".to_string(),
            foreground: "#CCCCCC".to_string(),
            background: "#1E1E1E".to_string(),
            cursor: "#FFFFFF".to_string(),
            selection: "#264F78".to_string(),
            black: "#000000".to_string(),
            red: "#CD3131".to_string(),
            green: "#0DBC79".to_string(),
            yellow: "#E5E510".to_string(),
            blue: "#2472C8".to_string(),
            magenta: "#BC3FBC".to_string(),
            cyan: "#11A8CD".to_string(),
            white: "#E5E5E5".to_string(),
            bright_black: "#666666".to_string(),
            bright_red: "#F14C4C".to_string(),
            bright_green: "#23D18B".to_string(),
            bright_yellow: "#F5F543".to_string(),
            bright_blue: "#3B8EEA".to_string(),
            bright_magenta: "#D670D6".to_string(),
            bright_cyan: "#29B8DB".to_string(),
            bright_white: "#FFFFFF".to_string(),
        }
    }

    fn now_string() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }

    pub async fn create_terminal(&self, options: TerminalCreateOptions) -> Result<TerminalInstance> {
        let id = Uuid::new_v4().to_string();
        let cwd = options.cwd.unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/".to_string())
        });
        let shell = options.shell.unwrap_or_else(Self::default_shell);
        let cols = options.cols.max(1);
        let rows = options.rows.max(1);
        let title = options.title.unwrap_or_else(|| "Terminal".to_string());

        let pty_size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty_pair = self.pty_system.lock().await.openpty(pty_size)?;
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(&cwd);
        
        if let Some(env) = options.env {
            for (k, v) in env {
                cmd.env(k, v);
            }
        }

        let child = pty_pair.slave.spawn_command(cmd)?;
        let pid = child.process_id();

        let writer = pty_pair.master.take_writer()?;
        let reader = pty_pair.master.try_clone_reader()?;

        let event_sender = self.event_sender.clone();
        let session_id = id.clone();
        
        let reader_task = tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            let mut reader = reader;
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let data = String::from_utf8_lossy(&buf[..n]).to_string();
                        let _ = event_sender.send((session_id.clone(), TerminalEvent::Data(data)));
                    }
                    Err(e) => {
                        eprintln!("Terminal read error: {}", e);
                        break;
                    }
                }
            }
            let _ = event_sender.send((session_id, TerminalEvent::Exit(-1)));
        });

        let created_at = Self::now_string();
        let last_activity = Self::now_string();

        let session = TerminalSession {
            id: id.clone(),
            pty_pair,
            writer: Some(writer),
            reader_task: Some(reader_task),
            cwd: cwd.clone(),
            shell: shell.clone(),
            cols,
            rows,
            title: title.clone(),
            pid,
            is_active: true,
            created_at: created_at.clone(),
            last_activity: last_activity.clone(),
            event_sender: self.event_sender.clone(),
        };

        let session = Arc::new(Mutex::new(session));
        self.sessions.write().await.insert(id.clone(), session.clone());

        Ok(TerminalInstance {
            id,
            title,
            cwd,
            shell,
            cols,
            rows,
            pid,
            is_active: true,
            created_at,
            last_activity,
        })
    }

    pub async fn get_terminal(&self, id: &str) -> Result<TerminalInstance> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        let session = session.lock().await;
        Ok(TerminalInstance {
            id: session.id.clone(),
            title: session.title.clone(),
            cwd: session.cwd.clone(),
            shell: session.shell.clone(),
            cols: session.cols,
            rows: session.rows,
            pid: session.pid,
            is_active: session.is_active,
            created_at: session.created_at.clone(),
            last_activity: session.last_activity.clone(),
        })
    }

    pub async fn list_terminals(&self) -> Result<Vec<TerminalInstance>> {
        let sessions = self.sessions.read().await;
        let mut terminals = Vec::new();
        for session in sessions.values() {
            let session = session.lock().await;
            terminals.push(TerminalInstance {
                id: session.id.clone(),
                title: session.title.clone(),
                cwd: session.cwd.clone(),
                shell: session.shell.clone(),
                cols: session.cols,
                rows: session.rows,
                pid: session.pid,
                is_active: session.is_active,
                created_at: session.created_at.clone(),
                last_activity: session.last_activity.clone(),
            });
        }
        Ok(terminals)
    }

    pub async fn write(&self, id: &str, data: String) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        let mut session = session.lock().await;
        
        if let Some(writer) = &mut session.writer {
            use std::io::Write;
            writer.write_all(data.as_bytes())?;
            writer.flush()?;
            session.last_activity = Self::now_string();
        }
        Ok(())
    }

    pub async fn resize(&self, id: &str, cols: u16, rows: u16) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        let mut session = session.lock().await;
        
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };
        session.pty_pair.master.resize(size)?;
        session.cols = cols;
        session.rows = rows;
        session.last_activity = Self::now_string();
        
        let _ = session.event_sender.send((session.id.clone(), TerminalEvent::Resize(cols, rows)));
        Ok(())
    }

    pub async fn kill(&self, id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session_arc) = sessions.remove(id) {
            let mut session = session_arc.lock().await;
            if let Some(task) = session.reader_task.take() {
                task.abort();
            }
            if let Some(mut writer) = session.writer.take() {
                let _ = writer.flush();
            }
            // Drop the pty_pair to close the PTY
            let _pty_pair = std::mem::replace(&mut session.pty_pair, unsafe { std::mem::zeroed() });
            drop(_pty_pair);
        }
        Ok(())
    }

    pub async fn set_title(&self, id: &str, title: String) -> Result<()> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        let mut session = session.lock().await;
        session.title = title.clone();
        session.last_activity = Self::now_string();
        let _ = session.event_sender.send((session.id.clone(), TerminalEvent::TitleChanged(title)));
        Ok(())
    }

    pub async fn get_profiles(&self) -> Result<Vec<TerminalProfile>> {
        Ok(self.profiles.read().await.clone())
    }

    pub async fn add_profile(&self, profile: TerminalProfile) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        profiles.push(profile);
        Ok(())
    }

    pub async fn update_profile(&self, name: &str, profile: TerminalProfile) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        if let Some(idx) = profiles.iter().position(|p| p.name == name) {
            profiles[idx] = profile;
            Ok(())
        } else {
            Err(TerminalError::InvalidConfig(format!("Profile not found: {}", name)))
        }
    }

    pub async fn delete_profile(&self, name: &str) -> Result<()> {
        let mut profiles = self.profiles.write().await;
        if let Some(idx) = profiles.iter().position(|p| p.name == name) {
            if profiles.len() > 1 {
                profiles.remove(idx);
                Ok(())
            } else {
                Err(TerminalError::InvalidConfig("Cannot delete the last profile".to_string()))
            }
        } else {
            Err(TerminalError::InvalidConfig(format!("Profile not found: {}", name)))
        }
    }

    pub async fn get_config(&self) -> Result<TerminalConfig> {
        Ok(self.config.read().await.clone())
    }

    pub async fn update_config(&self, config: TerminalConfig) -> Result<()> {
        *self.config.write().await = config;
        Ok(())
    }

    pub async fn detect_links(&self, _id: &str, text: &str, line: u32) -> Vec<TerminalLink> {
        let mut links = Vec::new();
        let url_regex = regex::Regex::new(r#"(https?://[^\s<>"{}|\\^`\[\]]+)"#).unwrap();
        
        for mat in url_regex.find_iter(text) {
            links.push(TerminalLink {
                text: mat.as_str().to_string(),
                url: mat.as_str().to_string(),
                line,
                start_col: mat.start() as u32,
                end_col: mat.end() as u32,
            });
        }
        
        let file_regex = regex::Regex::new(r#"([~/][^\s<>"{}|\\^`\[\]]*\.(?:rs|js|ts|py|go|java|cpp|c|h|hpp|json|toml|yaml|yml|md|txt))"#).unwrap();
        for mat in file_regex.find_iter(text) {
            links.push(TerminalLink {
                text: mat.as_str().to_string(),
                url: format!("file://{}", mat.as_str()),
                line,
                start_col: mat.start() as u32,
                end_col: mat.end() as u32,
            });
        }
        
        links
    }

    pub async fn get_process_info(&self, id: &str) -> Result<Vec<TerminalProcessInfo>> {
        let sessions = self.sessions.read().await;
        let session = sessions.get(id).ok_or_else(|| TerminalError::NotFound(id.to_string()))?;
        let session = session.lock().await;
        
        let mut processes = Vec::new();
        if let Some(pid) = session.pid {
            processes.push(TerminalProcessInfo {
                pid,
                name: session.shell.clone(),
                cwd: session.cwd.clone(),
                cmdline: vec![session.shell.clone()],
                cpu_usage: 0.0,
                memory_usage: 0,
            });
        }
        Ok(processes)
    }
}

impl Default for TerminalService {
    fn default() -> Self {
        let (service, _) = Self::new();
        service
    }
}