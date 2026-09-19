//! Lazy Sidecar Management
//!
//! Provides on-demand startup and idle shutdown for sidecar processes
//! (extension host, AI sidecar, etc.) to reduce resource usage.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager, Emitter};

/// Sidecar configuration
#[derive(Debug, Clone)]
pub struct SidecarConfig {
    pub name: String,
    pub script_name: String,
    pub idle_timeout: Duration,
    pub startup_timeout: Duration,
}

/// Sidecar state
pub struct SidecarState {
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<std::process::ChildStdin>>>,
    is_ready: Arc<Mutex<bool>>,
    last_activity: Arc<Mutex<Option<Instant>>>,
    config: SidecarConfig,
    app_handle: Option<AppHandle>,
}

impl SidecarState {
    pub fn new(config: SidecarConfig) -> Self {
        Self {
            process: Arc::new(Mutex::new(None)),
            stdin: Arc::new(Mutex::new(None)),
            is_ready: Arc::new(Mutex::new(false)),
            last_activity: Arc::new(Mutex::new(None)),
            config,
            app_handle: None,
        }
    }

    pub fn set_app_handle(&mut self, app: AppHandle) {
        self.app_handle = Some(app);
    }

    /// Check if sidecar is running and ready
    pub fn is_ready(&self) -> bool {
        *self.is_ready.lock()
    }

    /// Update last activity timestamp
    pub fn touch(&self) {
        *self.last_activity.lock() = Some(Instant::now());
    }

    /// Check if sidecar should be shut down due to idle timeout
    pub fn should_shutdown(&self) -> bool {
        let last_activity = *self.last_activity.lock();
        if let Some(last) = last_activity {
            last.elapsed() > self.config.idle_timeout
        } else {
            false
        }
    }

    /// Get the sidecar config
    pub fn config(&self) -> &SidecarConfig {
        &self.config
    }
}

/// Manager for multiple sidecars with lazy loading
pub struct SidecarManager {
    sidecars: Arc<Mutex<HashMap<String, Arc<SidecarState>>>>,
    shutdown_check_interval: Duration,
}

impl SidecarManager {
    pub fn new() -> Self {
        Self {
            sidecars: Arc::new(Mutex::new(HashMap::new())),
            shutdown_check_interval: Duration::from_secs(30),
        }
    }

    /// Register a sidecar configuration
    pub fn register(&self, name: String, config: SidecarConfig) {
        let sidecar = Arc::new(SidecarState::new(config));
        self.sidecars.lock().insert(name, sidecar);
    }

    /// Get or create a sidecar state
    pub fn get(&self, name: &str) -> Option<Arc<SidecarState>> {
        self.sidecars.lock().get(name).cloned()
    }

    /// Get a sidecar, starting it if necessary
    pub async fn get_or_start(
        &self,
        app: AppHandle,
        name: &str,
    ) -> Result<Arc<SidecarState>, String> {
        let sidecar = self
            .get(name)
            .ok_or_else(|| format!("Sidecar '{}' not registered", name))?;

        // Check if already running and ready
        if sidecar.is_ready() {
            sidecar.touch();
            return Ok(sidecar);
        }

        // Start the sidecar
        let sidecar_clone = Arc::clone(&sidecar);
        self.start_sidecar(app, sidecar_clone).await?;
        sidecar.touch();
        Ok(sidecar)
    }

    /// Start a sidecar process
    async fn start_sidecar(
        &self,
        app: AppHandle,
        sidecar: Arc<SidecarState>,
    ) -> Result<(), String> {
        let mut process_guard = sidecar.process.lock();

        // Double-check after acquiring lock
        if process_guard.is_some() {
            return Ok(());
        }

        // Find the sidecar script
        let sidecar_path = app
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource dir: {}", e))?
            .join("binaries")
            .join(&sidecar.config.script_name);

        // Fall back to development path
        let sidecar_path = if sidecar_path.exists() {
            sidecar_path
        } else {
            let dev_path = std::env::current_dir()
                .map_err(|e| format!("Failed to get current dir: {}", e))?
                .join("binaries")
                .join(&sidecar.config.script_name);

            if dev_path.exists() {
                dev_path
            } else {
                std::env::current_dir()
                    .map_err(|e| format!("Failed to get current dir: {}", e))?
                    .join("src-tauri")
                    .join("binaries")
                    .join(&sidecar.config.script_name)
            }
        };

        if !sidecar_path.exists() {
            return Err(format!(
                "Sidecar '{}' not found at: {:?}",
                sidecar.config.name, sidecar_path
            ));
        }

        // Spawn Node.js with the sidecar script
        let mut child = Command::new("node")
            .arg(&sidecar_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to spawn sidecar '{}': {}", sidecar.config.name, e))?;

        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;

        *process_guard = Some(child);
        *sidecar.stdin.lock() = Some(stdin);
        *sidecar.is_ready.lock() = false;
        *sidecar.last_activity.lock() = Some(Instant::now());

        // Spawn thread to read stdout
        let app_clone = app.clone();
        let is_ready = sidecar.is_ready.clone();
        let name = sidecar.config.name.clone();

        thread::spawn(move || {
            use std::io::{BufRead, BufReader};

            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if let Ok(response) = serde_json::from_str::<serde_json::Value>(&line) {
                            if let Some(msg_type) = response.get("type").and_then(|v| v.as_str()) {
                                if msg_type == "ready" {
                                    *is_ready.lock() = true;
                                    let _ = app_clone.emit(&format!("{}-ready", name), ());
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error reading sidecar '{}' output: {}", name, e);
                        break;
                    }
                }
            }

            *is_ready.lock() = false;
            let _ = app_clone.emit(&format!("{}-exit", name), ());
        });

        // Wait for ready signal
        let startup_timeout = sidecar.config.startup_timeout;
        let start = Instant::now();
        while !*sidecar.is_ready.lock() {
            if start.elapsed() > startup_timeout {
                return Err(format!(
                    "Sidecar '{}' startup timed out",
                    sidecar.config.name
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }

    /// Stop a specific sidecar
    pub fn stop(&self, name: &str) -> Result<(), String> {
        if let Some(sidecar) = self.get(name) {
            let mut process_guard = sidecar.process.lock();
            if let Some(mut child) = process_guard.take() {
                let _ = child.kill();
            }
            *sidecar.stdin.lock() = None;
            *sidecar.is_ready.lock() = false;
        }
        Ok(())
    }

    /// Stop all sidecars
    pub fn stop_all(&self) {
        let sidecars = self.sidecars.lock().clone();
        for (_, sidecar) in sidecars {
            let mut process_guard = sidecar.process.lock();
            if let Some(mut child) = process_guard.take() {
                let _ = child.kill();
            }
            *sidecar.stdin.lock() = None;
            *sidecar.is_ready.lock() = false;
        }
    }

    /// Check for idle sidecars and shut them down
    pub fn check_idle(&self) {
        let sidecars = self.sidecars.lock().clone();
        for (name, sidecar) in sidecars {
            if sidecar.should_shutdown() {
                log::info!("Shutting down idle sidecar: {}", name);
                let mut process_guard = sidecar.process.lock();
                if let Some(mut child) = process_guard.take() {
                    let _ = child.kill();
                }
                *sidecar.stdin.lock() = None;
                *sidecar.is_ready.lock() = false;
            }
        }
    }

    /// Start the idle checker background task
    pub fn start_idle_checker(&self) {
        let manager = self.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(manager.shutdown_check_interval);
                manager.check_idle();
            }
        });
    }
}

impl Clone for SidecarManager {
    fn clone(&self) -> Self {
        Self {
            sidecars: self.sidecars.clone(),
            shutdown_check_interval: self.shutdown_check_interval,
        }
    }
}

impl Default for SidecarManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Default sidecar configurations
pub fn default_sidecar_configs() -> Vec<(String, SidecarConfig)> {
    vec![
        (
            "extension-host".to_string(),
            SidecarConfig {
                name: "extension-host".to_string(),
                script_name: "extension-host-sidecar.js".to_string(),
                idle_timeout: Duration::from_secs(300), // 5 minutes
                startup_timeout: Duration::from_secs(30),
            },
        ),
        (
            "ai".to_string(),
            SidecarConfig {
                name: "ai".to_string(),
                script_name: "ai-sidecar.js".to_string(),
                idle_timeout: Duration::from_secs(600), // 10 minutes
                startup_timeout: Duration::from_secs(30),
            },
        ),
    ]
}

/// Tauri command to start a sidecar on demand
#[tauri::command]
pub async fn start_sidecar(
    app: AppHandle,
    manager: tauri::State<'_, SidecarManager>,
    name: String,
) -> Result<(), String> {
    manager.get_or_start(app, &name).await?;
    Ok(())
}

/// Tauri command to stop a sidecar
#[tauri::command]
pub fn stop_sidecar(
    manager: tauri::State<'_, SidecarManager>,
    name: String,
) -> Result<(), String> {
    manager.stop(&name)
}

/// Tauri command to check if sidecar is ready
#[tauri::command]
pub fn is_sidecar_ready(
    manager: tauri::State<'_, SidecarManager>,
    name: String,
) -> bool {
    manager
        .get(&name)
        .map(|s| s.is_ready())
        .unwrap_or(false)
}