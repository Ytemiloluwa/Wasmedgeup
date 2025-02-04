use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("wasmedgeup")
                .build()
                .unwrap(),
        }
    }

    pub async fn download_file(&self, url: &str, dest: &Path) -> Result<()> {
        println!("Downloading from: {}", url);

        let resp = self.client
            .get(url)
            .send()
            .await
            .context("Failed to send request")?;

        if !resp.status().is_success() {
            anyhow::bail!("Failed to download file: HTTP {}", resp.status());
        }

        let total_size = resp.content_length().unwrap_or(0);
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"));

        let mut file = File::create(dest).await.context("Failed to create file")?;
        let mut downloaded: u64 = 0;
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to download chunk")?;
            file.write_all(&chunk).await.context("Failed to write chunk")?;
            downloaded = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
            pb.set_position(downloaded);
        }

        pb.finish_with_message("Download completed");
        Ok(())
    }

    pub async fn verify_checksum(&self, file_path: &Path, expected_sha256: &str) -> Result<bool> {
        let mut file = File::open(file_path).await.context("Failed to open file for verification")?;
        let mut hasher = Sha256::new();
        let mut buffer = [0; 8192];

        loop {
            let n = tokio::io::AsyncReadExt::read(&mut file, &mut buffer).await?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let result = hex::encode(hasher.finalize());
        Ok(result == expected_sha256)
    }

    pub async fn download_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let response = self.client
            .get(url)
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Failed to download: HTTP {}", response.status());
        }

        let json = response.json().await?;
        Ok(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_download_file() {
        let downloader = Downloader::new();
        let temp_dir = tempdir().unwrap();
        let dest_path = temp_dir.path().join("test.txt");
        
        // Use a reliable test URL
        let url = "https://raw.githubusercontent.com/WasmEdge/WasmEdge/master/README.md";
        
        let result = downloader.download_file(url, &dest_path).await;
        assert!(result.is_ok());
        assert!(dest_path.exists());
    }
} 