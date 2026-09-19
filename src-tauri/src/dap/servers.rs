use crate::dap::client::{DapServerConfig, DebugSession, DebugSessionStatus};
use anyhow::Result;
use reqwest::Client as HttpClient;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use futures_util::StreamExt;
use flate2::read::GzDecoder;
use std::io::{Read, BufReader, Write};
use tracing::{info, warn};

#[derive(Clone)]
pub struct DapServerManager {
    servers_dir: PathBuf,
    http_client: Arc<HttpClient>,
    installed_servers: Arc<Mutex<HashMap<String, InstalledDapServer>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledDapServer {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub command: Vec<String>,
    pub args: Vec<String>,
    pub languages: Vec<String>,
    pub file_extensions: Vec<String>,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub last_used: chrono::DateTime<chrono::Utc>,
    pub auto_update: bool,
}

impl DapServerManager {
    pub fn new() -> Result<Self> {
        let servers_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("elixide")
            .join("dap_servers");

        std::fs::create_dir_all(&servers_dir)?;

        let manager = Self {
            servers_dir,
            http_client: Arc::new(HttpClient::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()?),
            installed_servers: Arc::new(Mutex::new(HashMap::new())),
        };

        manager.load_installed_servers()?;

        Ok(manager)
    }

    fn load_installed_servers(&self) -> Result<()> {
        let manifest_path = self.servers_dir.join("manifest.json");
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)?;
            let servers: HashMap<String, InstalledDapServer> = serde_json::from_str(&content)?;
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

    pub async fn ensure_server(&self, config: &DapServerConfig) -> Result<PathBuf> {
        let server_name = &config.name;

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

        self.download_and_install(config).await
    }

    async fn download_and_install(&self, config: &DapServerConfig) -> Result<PathBuf> {
        let server_name = &config.name;
        let server_dir = self.servers_dir.join(server_name);

        std::fs::create_dir_all(&server_dir)?;

        let binary_path = if let Some(download_url) = &config.download_url {
            info!("Downloading {} from {}", server_name, download_url);
            self.download_binary(download_url, &server_dir, server_name).await?
        } else {
            if let Ok(path) = which::which(&config.command[0]) {
                info!("Found {} in PATH: {}", server_name, path.display());
                let target = server_dir.join(config.command[0].clone());
                std::fs::copy(&path, &target)?;
                self.make_executable(&target)?;
                target
            } else {
                return Err(anyhow::anyhow!("No installation method for {}", server_name));
            }
        };

        self.verify_installation(&binary_path, &config.command[0])?;

        let installed = InstalledDapServer {
            name: server_name.clone(),
            version: self.get_version(&binary_path, &config.command[0]).await?,
            path: binary_path.clone(),
            command: config.command.clone(),
            args: config.args.clone(),
            languages: config.languages.clone(),
            file_extensions: config.file_extensions.clone(),
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

    pub fn get_installed_servers(&self) -> HashMap<String, InstalledDapServer> {
        self.installed_servers.lock().unwrap().clone()
    }

    pub async fn uninstall_server(&self, name: &str) -> Result<()> {
        let mut servers = self.installed_servers.lock().unwrap();
        if let Some(installed) = servers.remove(name) {
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

impl Default for DapServerManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_dap_server_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = DapServerManager::new().unwrap();
        assert!(manager.servers_dir.exists());
    }
}