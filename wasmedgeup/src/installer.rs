use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncReadExt;
use flate2::read::GzDecoder;
use tar::Archive;
use crate::{
    downloader::Downloader,
    platform::{Platform, OS},
};

const WASMEDGE_GITHUB_REPO: &str = "WasmEdge/WasmEdge";

pub struct Installer {
    install_path: PathBuf,
    temp_dir: PathBuf,
    platform: Platform,
    downloader: Downloader,
}

impl Installer {
    pub fn new(install_path: PathBuf, temp_dir: PathBuf, platform: Platform) -> Self {
        Self {
            install_path,
            temp_dir,
            platform,
            downloader: Downloader::new(),
        }
    }

    pub async fn install_runtime(&self, version: &str) -> Result<()> {
        // Create necessary directories
        fs::create_dir_all(&self.install_path).await?;
        fs::create_dir_all(&self.temp_dir).await?;

        // Prepare paths for installation
        let bin_dir = self.install_path.join("bin");
        let lib_dir = self.install_path.join("lib");
        let include_dir = self.install_path.join("include");
        let plugin_dir = self.install_path.join("plugin");

        fs::create_dir_all(&bin_dir).await?;
        fs::create_dir_all(&lib_dir).await?;
        fs::create_dir_all(&include_dir).await?;
        fs::create_dir_all(&plugin_dir).await?;

        // Download WasmEdge release
        let package_name = self.platform.get_release_package_name(version);
        let download_url = format!(
            "https://github.com/{}/releases/download/{}/WasmEdge-{}-{}",
            WASMEDGE_GITHUB_REPO, version, version, package_name
        );

        println!("Downloading from: {}", download_url);

        let archive_path = self.temp_dir.join(format!("wasmedge-{}.tar.gz", version));
        self.downloader.download_file(&download_url, &archive_path).await?;

        // Extract archive
        self.extract_archive(&archive_path).await?;

        // Set up environment variables
        self.setup_environment().await?;

        // Cleanup
        fs::remove_file(archive_path).await?;

        Ok(())
    }

    async fn extract_archive(&self, archive_path: &Path) -> Result<()> {
        let file_content = fs::read(archive_path).await.context("Failed to read archive file")?;
        let gz = GzDecoder::new(&file_content[..]);
        let mut archive = Archive::new(gz);
        
        // Extract to temp directory first
        archive.unpack(&self.temp_dir).context("Failed to extract archive")?;

        // Move files to their proper locations
        let extracted_dir = self.temp_dir.join(format!("WasmEdge-{}-{}", self.platform.os, self.platform.arch));
        
        println!("Extracting to: {}", extracted_dir.display());

        // Move bin files
        if let Ok(mut entries) = fs::read_dir(extracted_dir.join("bin")).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let target = self.install_path.join("bin").join(entry.file_name());
                fs::rename(entry.path(), target).await?;
            }
        }

        // Move lib files
        let lib_source = if extracted_dir.join("lib64").exists() {
            extracted_dir.join("lib64")
        } else {
            extracted_dir.join("lib")
        };

        if let Ok(mut entries) = fs::read_dir(lib_source).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let target = self.install_path.join("lib").join(entry.file_name());
                fs::rename(entry.path(), target).await?;
            }
        }

        // Move include files
        if let Ok(mut entries) = fs::read_dir(extracted_dir.join("include")).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let target = self.install_path.join("include").join(entry.file_name());
                fs::rename(entry.path(), target).await?;
            }
        }

        Ok(())
    }

    async fn setup_environment(&self) -> Result<()> {
        let env_file = self.install_path.join("env");
        let mut content = String::new();

        // Add environment variables based on OS
        match self.platform.os {
            OS::Linux(_) => {
                content.push_str("#!/bin/sh\n");
                content.push_str(&format!("export PATH={}:$PATH\n", self.install_path.join("bin").display()));
                content.push_str(&format!("export LD_LIBRARY_PATH={}:$LD_LIBRARY_PATH\n", self.install_path.join("lib").display()));
            }
            OS::Darwin => {
                content.push_str("#!/bin/sh\n");
                content.push_str(&format!("export PATH={}:$PATH\n", self.install_path.join("bin").display()));
                content.push_str(&format!("export DYLD_LIBRARY_PATH={}:$DYLD_LIBRARY_PATH\n", self.install_path.join("lib").display()));
            }
            OS::Windows => {
                // For Windows, the system PATH will need to be modified.
                // This will be handled differently in a real implementation
                content.push_str("@echo off\n");
                content.push_str(&format!("set PATH={};%PATH%\n", self.install_path.join("bin").display()));
            }
        }

        fs::write(env_file, content).await?;

        // Make the env file executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.install_path.join("env")).await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&self.install_path.join("env"), perms).await?;
        }

        Ok(())
    }

    pub async fn remove_runtime(&self) -> Result<()> {
        // Remove all files and directories
        if self.install_path.exists() {
            fs::remove_dir_all(&self.install_path).await?;
        }
        Ok(())
    }
} 