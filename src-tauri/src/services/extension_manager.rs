use anyhow::Result;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::fs;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tracing::{info, warn};
use futures_util::StreamExt;
use zip::ZipArchive;
use std::io::BufReader;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtensionInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub publisher: String,
    pub publisher_display_name: String,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub install_count: u64,
    pub latest_version: String,
    pub versions: Vec<ExtensionVersion>,
    pub icon: Option<String>,
    pub readme: Option<String>,
    pub changelog: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionVersion {
    pub version: String,
    pub last_updated: String,
    pub target_platform: String,
    pub asset_uri: String,
    pub size: u64,
    pub fallback_asset_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub extension_path: PathBuf,
    pub package_json: serde_json::Value,
    pub is_active: bool,
    pub installed_at: chrono::DateTime<chrono::Utc>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub extensions: Vec<ExtensionInfo>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Clone)]
pub struct ExtensionManager {
    extensions_dir: PathBuf,
    http_client: Arc<HttpClient>,
    installed_extensions: Arc<Mutex<HashMap<String, InstalledExtension>>>,
    openvsx_base_url: String,
}

impl ExtensionManager {
    pub fn new() -> Result<Self> {
        let extensions_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("elixide")
            .join("extensions");
        
        std::fs::create_dir_all(&extensions_dir)?;
        
        let manager = Self {
            extensions_dir,
            http_client: Arc::new(HttpClient::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()?),
            installed_extensions: Arc::new(Mutex::new(HashMap::new())),
            openvsx_base_url: "https://open-vsx.org/api".to_string(),
        };
        
        manager.load_installed_extensions()?;
        
        Ok(manager)
    }
    
    fn load_installed_extensions(&self) -> Result<()> {
        let manifest_path = self.extensions_dir.join("installed.json");
        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)?;
            let extensions: HashMap<String, InstalledExtension> = serde_json::from_str(&content)?;
            *self.installed_extensions.lock().unwrap() = extensions;
        }
        Ok(())
    }
    
    fn save_installed_extensions(&self) -> Result<()> {
        let manifest_path = self.extensions_dir.join("installed.json");
        let extensions = self.installed_extensions.lock().unwrap();
        let content = serde_json::to_string_pretty(&*extensions)?;
        std::fs::write(&manifest_path, content)?;
        Ok(())
    }
    
    pub async fn search_extensions(&self, query: &str, page: u32, page_size: u32) -> Result<SearchResult> {
        let url = format!("{}/search", self.openvsx_base_url);
        let response = self.http_client.get(&url)
            .query(&[("text", query), ("page", &page.to_string()), ("size", &page_size.to_string())])
            .header("User-Agent", "ElixirIDE")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Search failed: {}", response.status()));
        }
        
        let json: serde_json::Value = response.json().await?;
        let extensions = json["extensions"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
            .collect();
        
        Ok(SearchResult {
            extensions,
            total: json["total"].as_u64().unwrap_or(0),
            page,
            page_size,
        })
    }
    
    pub async fn get_extension_info(&self, publisher: &str, name: &str) -> Result<ExtensionInfo> {
        let url = format!("{}/{}/{}", self.openvsx_base_url, publisher, name);
        let response = self.http_client.get(&url)
            .header("User-Agent", "ElixirIDE")
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get extension info: {}", response.status()));
        }
        
        let json: serde_json::Value = response.json().await?;
        let info: ExtensionInfo = serde_json::from_value(json)?;
        Ok(info)
    }
    
    pub async fn install_extension(&self, publisher: &str, name: &str, version: Option<&str>) -> Result<InstalledExtension> {
        let info = self.get_extension_info(publisher, name).await?;
        
        let version_to_install = version.unwrap_or(&info.latest_version);
        let version_info = info.versions.iter()
            .find(|v| v.version == version_to_install)
            .ok_or_else(|| anyhow::anyhow!("Version {} not found", version_to_install))?;
        
        info!("Installing extension {}.{}@{}", publisher, name, version_to_install);
        
        let download_url = &version_info.asset_uri;
        let extension_dir = self.extensions_dir.join(format!("{}.{}", publisher, name));
        std::fs::create_dir_all(&extension_dir)?;
        
        // Download and extract
        self.download_and_extract(download_url, &extension_dir).await?;
        
        // Read package.json
        let package_json_path = extension_dir.join("package.json");
        let package_content = std::fs::read_to_string(&package_json_path)?;
        let package_json: serde_json::Value = serde_json::from_str(&package_content)?;
        
        let installed = InstalledExtension {
            name: name.to_string(),
            version: version_to_install.to_string(),
            publisher: publisher.to_string(),
            extension_path: extension_dir.clone(),
            package_json,
            is_active: false,
            installed_at: chrono::Utc::now(),
            enabled: true,
        };
        
        let key = format!("{}.{}", publisher, name);
        {
            let mut extensions = self.installed_extensions.lock().unwrap();
            extensions.insert(key.clone(), installed.clone());
        }
        self.save_installed_extensions()?;
        
        info!("Successfully installed {}.{}@{}", publisher, name, version_to_install);
        Ok(installed)
    }
    
    async fn download_and_extract(&self, url: &str, dest: &Path) -> Result<()> {
        let response = self.http_client.get(url).send().await?;
        let total_size = response.content_length().unwrap_or(0);
        
        let file_name = if url.ends_with(".vsix") { "extension.vsix" } else { "extension.vsix" };
        let file_path = dest.join(file_name);
        
        let mut file = fs::File::create(&file_path).await?;
        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            
            if total_size > 0 && downloaded % (1024 * 1024) == 0 {
                info!("Downloaded {}/{} MB", downloaded / 1024 / 1024, total_size / 1024 / 1024);
            }
        }
        
        file.flush().await?;
        
        // Extract VSIX (it's a zip file)
        self.extract_vsix(&file_path, dest)?;
        fs::remove_file(&file_path).await?;
        
        Ok(())
    }
    
    fn extract_vsix(&self, vsix_path: &Path, dest: &Path) -> Result<()> {
        use zip::ZipArchive;
        use std::io::BufReader;
        
        let file = std::fs::File::open(vsix_path)?;
        let mut archive = ZipArchive::new(BufReader::new(file))?;
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = dest.join(file.mangled_name());
            
            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }
        
        Ok(())
    }
    
    pub async fn uninstall_extension(&self, publisher: &str, name: &str) -> Result<()> {
        let key = format!("{}.{}", publisher, name);
        
        {
            let mut extensions = self.installed_extensions.lock().unwrap();
            if let Some(installed) = extensions.remove(&key) {
                if installed.extension_path.exists() {
                    std::fs::remove_dir_all(&installed.extension_path).ok();
                }
            }
        }
        self.save_installed_extensions()?;
        
        info!("Uninstalled {}.{}", publisher, name);
        Ok(())
    }
    
    pub async fn enable_extension(&self, publisher: &str, name: &str) -> Result<()> {
        let key = format!("{}.{}", publisher, name);
        let mut extensions = self.installed_extensions.lock().unwrap();
        if let Some(ext) = extensions.get_mut(&key) {
            ext.enabled = true;
            drop(extensions);
            self.save_installed_extensions()?;
        }
        Ok(())
    }
    
    pub async fn disable_extension(&self, publisher: &str, name: &str) -> Result<()> {
        let key = format!("{}.{}", publisher, name);
        let mut extensions = self.installed_extensions.lock().unwrap();
        if let Some(ext) = extensions.get_mut(&key) {
            ext.enabled = false;
            drop(extensions);
            self.save_installed_extensions()?;
        }
        Ok(())
    }
    
    pub fn get_installed_extensions(&self) -> Vec<InstalledExtension> {
        self.installed_extensions.lock().unwrap().values().cloned().collect()
    }
    
    pub fn get_extension(&self, publisher: &str, name: &str) -> Option<InstalledExtension> {
        let key = format!("{}.{}", publisher, name);
        self.installed_extensions.lock().unwrap().get(&key).cloned()
    }
    
    pub fn get_activation_events(&self, publisher: &str, name: &str) -> Vec<String> {
        if let Some(ext) = self.get_extension(publisher, name) {
            if let Some(contributes) = ext.package_json.get("contributes") {
                if let Some(activation_events) = contributes.get("activationEvents") {
                    return activation_events.as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
        }
        vec![]
    }
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_extension_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = ExtensionManager::new().unwrap();
        assert!(manager.extensions_dir.exists());
    }
}