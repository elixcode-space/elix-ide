use crate::lsp::client::LSPServerConfig;
use anyhow::Result;
use reqwest::Client as HttpClient;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tracing::{info, warn};
use futures_util::StreamExt;

pub struct LSPServerManager {
    servers_dir: PathBuf,
    http_client: Arc<HttpClient>,
    installed_servers: Arc<Mutex<HashMap<String, InstalledServer>>>,
}

impl Clone for LSPServerManager {
    fn clone(&self) -> Self {
        Self {
            servers_dir: self.servers_dir.clone(),
            http_client: self.http_client.clone(),
            installed_servers: self.installed_servers.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledServer {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub command: Vec<String>,
    pub languages: Vec<String>,
    pub file_extensions: Vec<String>,
    pub root_patterns: Vec<String>,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub auto_update: bool,
}

impl LSPServerManager {
    pub fn new() -> Result<Self> {
        let servers_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("elixide")
            .join("lsp_servers");
        
        std::fs::create_dir_all(&servers_dir)?;
        
        let manager = Self {
            servers_dir,
            http_client: Arc::new(HttpClient::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()?),
            installed_servers: Arc::new(Mutex::new(HashMap::new())),
        };
        
        // Load installed servers from disk
        manager.load_installed_servers()?;
        
        Ok(manager)
    }
    
    fn load_installed_servers(&self) -> Result<()> {
        let manifest_path = self.servers_dir.join("manifest.json");
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)?;
            let servers: HashMap<String, InstalledServer> = serde_json::from_str(&content)?;
            *self.installed_servers.lock().unwrap() = servers;
        }
        Ok(())
    }
    
    fn save_installed_servers(&self) -> Result<()> {
        let manifest_path = self.servers_dir.join("manifest.json");
        let servers = self.installed_servers.lock().unwrap();
        let content = serde_json::to_string_pretty(&*servers)?;
        std::fs::write(&manifest_path, content)?;
        Ok(())
    }
    
    pub async fn ensure_server(&self, config: &LSPServerConfig) -> Result<PathBuf> {
        let server_name = &config.name;
        
        // Check if already installed
        let already_installed = {
            let servers = self.installed_servers.lock().unwrap();
            if let Some(installed) = servers.get(server_name) {
                installed.path.exists()
            } else {
                false
            }
        };
        
        if already_installed {
            self.update_last_used(server_name).await?;
            let servers = self.installed_servers.lock().unwrap();
            return Ok(servers.get(server_name).unwrap().path.clone());
        }
        
        // Download and install
        self.download_and_install(config).await
    }
    
    async fn download_and_install(&self, config: &LSPServerConfig) -> Result<PathBuf> {
        let server_name = &config.name;
        let server_dir = self.servers_dir.join(server_name);
        
        std::fs::create_dir_all(&server_dir)?;
        
        let binary_path = if let Some(download_url) = &config.download_url {
            // Download from URL
            info!("Downloading {} from {}", server_name, download_url);
            self.download_binary(download_url, &server_dir, server_name).await?
        } else {
            // Try to find in PATH
            if let Ok(path) = which::which(&config.command[0]) {
                info!("Found {} in PATH: {}", server_name, path.display());
                // Copy to our directory for consistent management
                let target = server_dir.join(config.command[0].clone());
                std::fs::copy(&path, &target)?;
                self.make_executable(&target)?;
                target
            } else {
                // Try npm install for Node-based servers
                if config.name.contains("typescript") || config.name.contains("pyright") {
                    self.install_via_npm(config, &server_dir).await?
                } else if config.name == "gopls" {
                    self.install_via_go(config, &server_dir).await?
                } else {
                    return Err(anyhow::anyhow!("No installation method for {}", server_name));
                }
            }
        };
        
        // Verify installation
        self.verify_installation(&binary_path, &config.command[0])?;
        
        // Record installation
        let installed = InstalledServer {
            name: server_name.clone(),
            version: self.get_version(&binary_path, &config.command[0]).await?,
            path: binary_path.clone(),
            command: config.command.clone(),
            languages: config.languages.clone(),
            file_extensions: config.file_extensions.clone(),
            root_patterns: config.root_patterns.clone(),
            installed_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
            auto_update: true,
        };
        
        {
            let mut servers = self.installed_servers.lock().unwrap();
            servers.insert(server_name.clone(), installed);
        }
        self.save_installed_servers()?;
        
        info!("Successfully installed {}", server_name);
        Ok(binary_path)
    }
    
    async fn download_binary(&self, url: &str, dir: &Path, name: &str) -> Result<PathBuf> {
        let response = self.http_client.get(url).send().await?;
        let total_size = response.content_length().unwrap_or(0);
        
        let filename = if url.ends_with(".gz") {
            format!("{}.gz", name)
        } else {
            name.to_string()
        };
        
        let file_path = dir.join(&filename);
        let mut file = fs::File::create(&file_path).await?;
        let mut downloaded = 0u64;
        
        let mut stream = response.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            
            if total_size > 0 && downloaded % (1024 * 1024) == 0 {
                info!("Downloaded {}/{} MB for {}", downloaded / 1024 / 1024, total_size / 1024 / 1024, name);
            }
        }
        
        file.flush().await?;
        
        // Extract if gzipped
        if filename.ends_with(".gz") {
            let extracted_path = dir.join(name);
            self.extract_gz(&file_path, &extracted_path)?;
            fs::remove_file(&file_path).await?;
            self.make_executable(&extracted_path)?;
            Ok(extracted_path)
        } else {
            self.make_executable(&file_path)?;
            Ok(file_path)
        }
    }
    
    fn extract_gz(&self, gz_path: &Path, out_path: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::io::{Read, BufReader, Write};
        
        let file = std::fs::File::open(gz_path)?;
        let mut decoder = GzDecoder::new(BufReader::new(file));
        let mut output = std::fs::File::create(out_path)?;
        let mut buffer = [0; 8192];
        
        loop {
            let n = decoder.read(&mut buffer)?;
            if n == 0 { break; }
            output.write_all(&buffer[..n])?;
        }
        
        Ok(())
    }
    
    async fn install_via_npm(&self, config: &LSPServerConfig, dir: &Path) -> Result<PathBuf> {
        info!("Installing {} via npm", config.name);
        
        let package_name = match config.name.as_str() {
            "typescript-language-server" => "typescript-language-server",
            "pyright-langserver" => "pyright",
            _ => return Err(anyhow::anyhow!("Unknown npm package for {}", config.name)),
        };
        
        // npm install -g --prefix <dir> <package>
        let output = Command::new("npm")
            .args(["install", "-g", "--prefix", dir.to_str().unwrap(), package_name])
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("npm install failed: {}", stderr));
        }
        
        let binary_name = if config.name == "typescript-language-server" {
            "typescript-language-server"
        } else {
            "pyright-langserver"
        };
        
        let binary_path = dir.join("bin").join(binary_name);
        if binary_path.exists() {
            self.make_executable(&binary_path)?;
            Ok(binary_path)
        } else {
            // Try node_modules/.bin
            let alt_path = dir.join("lib").join("node_modules").join(".bin").join(binary_name);
            if alt_path.exists() {
                self.make_executable(&alt_path)?;
                Ok(alt_path)
            } else {
                Err(anyhow::anyhow!("Binary not found after npm install"))
            }
        }
    }
    
    async fn install_via_go(&self, config: &LSPServerConfig, dir: &Path) -> Result<PathBuf> {
        info!("Installing {} via go install", config.name);
        
        let output = Command::new("go")
            .args(["install", "golang.org/x/tools/gopls@latest"])
            .env("GOBIN", dir.to_str().unwrap())
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("go install failed: {}", stderr));
        }
        
        let binary_path = dir.join("gopls");
        if binary_path.exists() {
            self.make_executable(&binary_path)?;
            Ok(binary_path)
        } else {
            Err(anyhow::anyhow!("gopls binary not found after go install"))
        }
    }
    
    fn make_executable(&self, path: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms)?;
        }
        Ok(())
    }
    
    fn verify_installation(&self, binary_path: &Path, binary_name: &str) -> Result<()> {
        if !binary_path.exists() {
            return Err(anyhow::anyhow!("Binary not found: {}", binary_path.display()));
        }
        
        // Try running with --version
        let output = Command::new(binary_path)
            .arg("--version")
            .output()?;
        
        if !output.status.success() {
            warn!("Binary {} exists but --version failed", binary_name);
        } else {
            let version = String::from_utf8_lossy(&output.stdout);
            info!("Verified {}: {}", binary_name, version.trim());
        }
        
        Ok(())
    }
    
    async fn get_version(&self, binary_path: &Path, binary_name: &str) -> Result<String> {
        let output = Command::new(binary_path)
            .arg("--version")
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("unknown".to_string())
        }
    }
    
    async fn update_last_used(&self, server_name: &str) -> Result<()> {
        let mut servers = self.installed_servers.lock().unwrap();
        if let Some(server) = servers.get_mut(server_name) {
            server.last_used = chrono::Utc::now();
            drop(servers);
            self.save_installed_servers()?;
        }
        Ok(())
    }
    
    pub async fn check_updates(&self) -> Result<Vec<ServerUpdate>> {
        let mut updates = Vec::new();
        
        // Clone the data we need while holding the lock briefly
        let servers_to_check: Vec<(String, String, bool)> = {
            let servers = self.installed_servers.lock().unwrap();
            servers.iter()
                .filter(|(_, installed)| installed.auto_update)
                .map(|(name, installed)| (name.clone(), installed.version.clone(), installed.auto_update))
                .collect()
        };
        
        for (name, current_version, _) in servers_to_check {
            // Check for updates based on server type
            if let Ok(latest_version) = self.fetch_latest_version(&name).await {
                if latest_version != current_version {
                    updates.push(ServerUpdate {
                        name: name.clone(),
                        current_version,
                        latest_version,
                        download_url: self.get_download_url(&name),
                    });
                }
            }
        }
        
        Ok(updates)
    }
    
    async fn fetch_latest_version(&self, name: &str) -> Result<String> {
        match name {
            "rust-analyzer" => {
                let url = "https://api.github.com/repos/rust-lang/rust-analyzer/releases/latest";
                let response = self.http_client.get(url)
                    .header("User-Agent", "ElixirIDE")
                    .send()
                    .await?;
                let json: serde_json::Value = response.json().await?;
                Ok(json["tag_name"].as_str().unwrap_or("unknown").to_string())
            }
            "gopls" => {
                // gopls version from go list
                Ok("latest".to_string())
            }
            _ => Ok("unknown".to_string()),
        }
    }
    
    fn get_download_url(&self, name: &str) -> Option<String> {
        match name {
            "rust-analyzer" => Some("https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu.gz".to_string()),
            _ => None,
        }
    }
    
    pub async fn update_server(&self, name: &str) -> Result<()> {
        let config = self.get_server_config(name)?;
        self.download_and_install(&config).await?;
        Ok(())
    }
    
    fn get_server_config(&self, name: &str) -> Result<LSPServerConfig> {
        // Return default configs for known servers
        match name {
            "rust-analyzer" => Ok(LSPServerConfig {
                name: "rust-analyzer".to_string(),
                command: vec!["rust-analyzer".to_string()],
                languages: vec!["rust".to_string()],
                file_extensions: vec!["rs".to_string()],
                root_patterns: vec!["Cargo.toml".to_string()],
                download_url: Some("https://github.com/rust-lang/rust-analyzer/releases/latest/download/rust-analyzer-x86_64-unknown-linux-gnu.gz".to_string()),
            }),
            "typescript-language-server" => Ok(LSPServerConfig {
                name: "typescript-language-server".to_string(),
                command: vec!["typescript-language-server".to_string(), "--stdio".to_string()],
                languages: vec!["typescript".to_string(), "javascript".to_string(), "typescriptreact".to_string(), "javascriptreact".to_string()],
                file_extensions: vec!["ts".to_string(), "js".to_string(), "tsx".to_string(), "jsx".to_string()],
                root_patterns: vec!["package.json".to_string(), "tsconfig.json".to_string()],
                download_url: None,
            }),
            "pyright-langserver" => Ok(LSPServerConfig {
                name: "pyright-langserver".to_string(),
                command: vec!["pyright-langserver".to_string(), "--stdio".to_string()],
                languages: vec!["python".to_string()],
                file_extensions: vec!["py".to_string()],
                root_patterns: vec!["pyproject.toml".to_string(), "setup.py".to_string(), "requirements.txt".to_string()],
                download_url: None,
            }),
            "gopls" => Ok(LSPServerConfig {
                name: "gopls".to_string(),
                command: vec!["gopls".to_string()],
                languages: vec!["go".to_string()],
                file_extensions: vec!["go".to_string()],
                root_patterns: vec!["go.mod".to_string()],
                download_url: None,
            }),
            _ => Err(anyhow::anyhow!("Unknown server: {}", name)),
        }
    }
    
    pub fn get_installed_servers(&self) -> HashMap<String, InstalledServer> {
        self.installed_servers.lock().unwrap().clone()
    }
    
    pub async fn uninstall_server(&self, name: &str) -> Result<()> {
        let mut servers = self.installed_servers.lock().unwrap();
        if let Some(installed) = servers.remove(name) {
            // Remove directory
            if installed.path.exists() {
                let server_dir = installed.path.parent().unwrap();
                if server_dir.starts_with(&self.servers_dir) {
                    std::fs::remove_dir_all(server_dir).ok();
                }
            }
            self.save_installed_servers()?;
            info!("Uninstalled {}", name);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ServerUpdate {
    pub name: String,
    pub current_version: String,
    pub latest_version: String,
    pub download_url: Option<String>,
}

impl Default for LSPServerManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_server_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = LSPServerManager::new().unwrap();
        assert!(manager.servers_dir.exists());
    }
}