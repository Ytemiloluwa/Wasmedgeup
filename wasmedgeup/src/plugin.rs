use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use crate::{
    platform::Platform,
    downloader::Downloader,
};
use log::{info, warn, error};
use flate2::read::GzDecoder;
use tar::Archive;
use tokio::fs;

const KNOWN_PLUGINS: &[&str] = &[
    "wasi-nn-ggml",
    "wasi-nn-pytorch",
    "wasi-nn-tensorflow",
    "wasi-crypto",
    "wasmedge-tensorflow",
    "wasmedge-tensorflowlite",
    "wasmedge-image",
];

#[derive(Debug, Deserialize)]
pub struct PluginVersionInfo {
    pub deps: Vec<String>,
    pub platform: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PluginManifest {
    #[serde(flatten)]
    pub plugins: HashMap<String, HashMap<String, PluginVersionInfo>>,
}

#[derive(Debug, Deserialize)]
pub struct VersionManifest {
    pub maintained: Vec<String>,
    pub deprecated: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    assets: Vec<ReleaseAsset>,
}

pub struct PluginManager {
    runtime_version: String,
    platform: Platform,
    downloader: Downloader,
}

impl PluginManager {
    pub fn new(runtime_version: String, platform: Platform) -> Self {
        Self {
            runtime_version,
            platform,
            downloader: Downloader::new(),
        }
    }

    fn get_platform_string(&self) -> String {
        match &self.platform.os {
            crate::platform::OS::Linux(distro) => match distro {
                crate::platform::LinuxDistro::Ubuntu => format!("ubuntu20_04_{}", self.platform.arch),
                _ => format!("manylinux_2_28_{}", self.platform.arch),
            },
            crate::platform::OS::Darwin => format!("darwin_{}", self.platform.arch),
            crate::platform::OS::Windows => format!("windows_{}", self.platform.arch),
        }
    }

    async fn fetch_version_manifest(&self, repo: &str) -> Result<VersionManifest> {
        let url = format!(
            "https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/plugins/{}/version.json",
            repo
        );
        info!("Fetching version manifest from: {}", url);
        self.downloader.download_json(&url).await
    }

    async fn fetch_plugin_manifest(&self, repo: &str, version: &str) -> Result<PluginManifest> {
        let url = format!(
            "https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/plugins/{}/manifest.json",
            repo
        );
        info!("Fetching plugin manifest from: {}", url);
        self.downloader.download_json(&url).await
    }

    pub async fn list_available_plugins(&self) -> Result<Vec<(String, String, bool)>> {
        let mut available_plugins = Vec::new();
        let platform_string = self.get_platform_string();

        // Fetch release information from GitHub API
        let url = format!(
            "https://api.github.com/repos/WasmEdge/WasmEdge/releases/tags/{}",
            self.runtime_version
        );
        info!("Fetching release information from: {}", url);
        
        let release: Release = self.downloader.download_json(&url).await?;
        let mut seen_plugins = HashSet::new();

        // Process plugin assets
        for asset in release.assets {
            if asset.name.starts_with("WasmEdge-plugin-") && asset.name.ends_with(".tar.gz") {
                // Extract plugin name and check platform compatibility
                let parts: Vec<&str> = asset.name.split('-').collect();
                if parts.len() >= 4 {
                    let plugin_name = parts[2..parts.len()-2].join("-");
                    let version = self.runtime_version.clone();
                    let is_compatible = asset.name.contains(&platform_string);

                    // Only add each plugin once
                    if !seen_plugins.contains(&plugin_name) {
                        seen_plugins.insert(plugin_name.clone());
                        available_plugins.push((plugin_name, version, is_compatible));
                    }
                }
            }
        }

        Ok(available_plugins)
    }

    async fn extract_plugin(&self, archive_path: &Path, plugin_dir: &Path) -> Result<()> {
        let file = std::fs::File::open(archive_path)?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        // Extract all .so files from the archive
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if let Some(ext) = path.extension() {
                if ext == "so" || ext == "dll" || ext == "dylib" {
                    let file_name = path.file_name().unwrap();
                    let dest_path = plugin_dir.join(file_name);
                    entry.unpack(&dest_path)?;
                    info!("Extracted plugin file: {}", dest_path.display());
                }
            }
        }

        Ok(())
    }

    pub async fn install_plugin(&self, plugin_name: &str, version: Option<String>) -> Result<()> {
        info!("Installing plugin {} (version: {:?})", plugin_name, version);

        let platform_string = self.get_platform_string();
        let mut installed = false;

        let plugin_dir = dirs::home_dir()
            .context("Could not determine home directory")?
            .join(".wasmedge")
            .join("plugin");

        std::fs::create_dir_all(&plugin_dir)?;

        // Format the plugin name for the URL (replace first - with _)
        let url_plugin_name = if let Some(pos) = plugin_name.find('-') {
            format!("{}_{}",
                &plugin_name[..pos],
                &plugin_name[pos + 1..]
            )
        } else {
            plugin_name.to_string()
        };

        // The plugins are distributed as separate archives
        let url = format!(
            "https://github.com/WasmEdge/WasmEdge/releases/download/{}/WasmEdge-plugin-{}-{}-{}.tar.gz",
            self.runtime_version, url_plugin_name, self.runtime_version, platform_string
        );

        let temp_dir = tempfile::tempdir()?;
        let archive_path = temp_dir.path().join("plugin.tar.gz");

        info!("Downloading plugin from: {}", url);
        match self.downloader.download_file(&url, &archive_path).await {
            Ok(_) => {
                info!("Successfully downloaded plugin archive");
                let file = std::fs::File::open(&archive_path)?;
                let gz = GzDecoder::new(file);
                let mut archive = Archive::new(gz);

                // Extract all plugin files from the archive
                for entry in archive.entries()? {
                    let mut entry = entry?;
                    let path = entry.path()?;
                    if let Some(ext) = path.extension() {
                        if ext == "so" || ext == "dll" || ext == "dylib" {
                            let file_name = path.file_name().unwrap();
                            let dest_path = plugin_dir.join(file_name);
                            entry.unpack(&dest_path)?;
                            info!("Extracted plugin file: {}", dest_path.display());
                            installed = true;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to download plugin: {}", e);
            }
        }

        if !installed {
            anyhow::bail!("Failed to install plugin. The plugin may not be available for your platform or the specified version.");
        }

        info!("Successfully installed plugin {}", plugin_name);
        Ok(())
    }

    pub fn remove_plugin(&self, plugin_name: &str, version: Option<String>) -> Result<()> {
        info!("Removing plugin {} (version: {:?})", plugin_name, version);

        let plugin_dir = dirs::home_dir()
            .context("Could not determine home directory")?
            .join(".wasmedge")
            .join("plugin");

        let pattern = match &version {
            Some(v) => format!("libwasmedge_{}-{}", plugin_name.replace('-', "_"), v),
            None => format!("libwasmedge_{}", plugin_name.replace('-', "_")),
        };

        let mut found = false;
        // Remove matching plugin files
        for entry in std::fs::read_dir(plugin_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            
            if file_name.contains(&pattern) {
                std::fs::remove_file(entry.path())?;
                info!("Removed plugin file: {}", file_name);
                found = true;
            }
        }

        if !found {
            anyhow::bail!("No matching plugin files found for {} (version: {:?})", plugin_name, version);
        }

        Ok(())
    }
} 