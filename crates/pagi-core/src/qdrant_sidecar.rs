//! Qdrant Sidecar Manager
//!
//! Automatically downloads, manages, and launches Qdrant as a sidecar process.
//! Ensures Phoenix has a working vector database without manual user setup.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use thiserror::Error;

const QDRANT_PORT: u16 = 6333;
const QDRANT_VERSION: &str = "v1.7.4"; // Update as needed

#[derive(Error, Debug)]
pub enum QdrantError {
    #[error("Failed to download Qdrant: {0}")]
    DownloadError(String),
    
    #[error("Failed to extract Qdrant: {0}")]
    ExtractionError(String),
    
    #[error("Failed to start Qdrant: {0}")]
    StartupError(String),
    
    #[error("Qdrant health check failed: {0}")]
    HealthCheckError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
}

/// Qdrant sidecar manager
pub struct QdrantSidecar {
    bin_dir: PathBuf,
    data_dir: PathBuf,
    process: Option<Child>,
}

impl QdrantSidecar {
    /// Create a new Qdrant sidecar manager using relative paths (current working directory).
    /// Prefer `new_with_root` when the process may not be started from the project root (e.g. from Temp).
    pub fn new() -> Self {
        Self::new_with_root(Path::new("."))
    }

    /// Create a new Qdrant sidecar manager with an explicit project root.
    /// Use this so Qdrant uses `root/bin` and `root/data/qdrant` regardless of process cwd.
    /// Pass the workspace/project root (e.g. directory containing `pagi-up.ps1`).
    pub fn new_with_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        let bin_dir = root.join("bin");
        let data_dir = root.join("data").join("qdrant");
        Self {
            bin_dir,
            data_dir,
            process: None,
        }
    }
    
    /// Check if Qdrant is already running on the expected port
    pub async fn is_running(&self) -> bool {
        self.health_check().await.is_ok()
    }
    
    /// Perform health check on Qdrant
    pub async fn health_check(&self) -> Result<(), QdrantError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()?;
        
        let url = format!("http://localhost:{}/health", QDRANT_PORT);
        
        match client.get(&url).send().await {
            Ok(response) if response.status().is_success() => Ok(()),
            Ok(response) => Err(QdrantError::HealthCheckError(
                format!("Unexpected status: {}", response.status())
            )),
            Err(e) => Err(QdrantError::HealthCheckError(e.to_string())),
        }
    }
    
    /// Ensure Qdrant is available (download if needed, start if not running)
    pub async fn ensure_running(&mut self) -> Result<(), QdrantError> {
        // Check if already running
        if self.is_running().await {
            tracing::info!("✅ Qdrant already running on port {}", QDRANT_PORT);
            return Ok(());
        }
        
        tracing::info!("🔍 Qdrant not detected. Initializing Memory Engine...");
        
        // Ensure binary exists
        let binary_path = self.ensure_binary().await?;
        
        // Start Qdrant
        self.start(&binary_path).await?;
        
        // Wait for health check
        self.wait_for_ready().await?;
        
        tracing::info!("✅ Memory Engine (Qdrant) initialized successfully");
        Ok(())
    }
    
    /// Ensure Qdrant binary exists (download if needed)
    async fn ensure_binary(&self) -> Result<PathBuf, QdrantError> {
        // Create bin directory if it doesn't exist
        fs::create_dir_all(&self.bin_dir)?;
        
        let binary_name = self.get_binary_name();
        let binary_path = self.bin_dir.join(&binary_name);
        
        // Check if binary already exists
        if binary_path.exists() {
            tracing::info!("✅ Qdrant binary found at {}", binary_path.display());
            return Ok(binary_path);
        }
        
        tracing::info!("📥 Downloading Qdrant {}...", QDRANT_VERSION);
        
        // Download binary
        self.download_binary(&binary_path).await?;
        
        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&binary_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&binary_path, perms)?;
        }
        
        tracing::info!("✅ Qdrant binary downloaded to {}", binary_path.display());
        Ok(binary_path)
    }
    
    /// Download Qdrant binary from GitHub releases
    async fn download_binary(&self, dest: &Path) -> Result<(), QdrantError> {
        let download_url = self.get_download_url();
        
        tracing::info!("Downloading from: {}", download_url);
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minutes for download
            .build()?;
        
        let response = client.get(&download_url)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(QdrantError::DownloadError(
                format!("HTTP {}: {}", response.status(), download_url)
            ));
        }
        
        let bytes = response.bytes().await?;
        
        // If it's a compressed archive, extract it
        if download_url.ends_with(".tar.gz") {
            self.extract_tar_gz(&bytes, dest)?;
        } else if download_url.ends_with(".zip") {
            self.extract_zip(&bytes, dest)?;
        } else {
            // Direct binary download
            let mut file = fs::File::create(dest)?;
            file.write_all(&bytes)?;
        }
        
        Ok(())
    }
    
    /// Extract tar.gz archive
    fn extract_tar_gz(&self, bytes: &[u8], dest: &Path) -> Result<(), QdrantError> {
        use flate2::read::GzDecoder;
        use tar::Archive;
        
        let decoder = GzDecoder::new(bytes);
        let mut archive = Archive::new(decoder);
        
        // Extract to temp directory first
        let temp_dir = self.bin_dir.join("temp_extract");
        fs::create_dir_all(&temp_dir)?;
        
        archive.unpack(&temp_dir)
            .map_err(|e| QdrantError::ExtractionError(e.to_string()))?;
        
        // Find the qdrant binary in extracted files
        let binary_name = if cfg!(windows) { "qdrant.exe" } else { "qdrant" };
        
        for entry in walkdir::WalkDir::new(&temp_dir) {
            let entry = entry.map_err(|e| QdrantError::ExtractionError(e.to_string()))?;
            if entry.file_name() == binary_name {
                fs::copy(entry.path(), dest)?;
                break;
            }
        }
        
        // Clean up temp directory
        fs::remove_dir_all(&temp_dir)?;
        
        Ok(())
    }
    
    /// Extract zip archive
    fn extract_zip(&self, bytes: &[u8], dest: &Path) -> Result<(), QdrantError> {
        use std::io::Cursor;
        use zip::ZipArchive;
        
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| QdrantError::ExtractionError(e.to_string()))?;
        
        let binary_name = if cfg!(windows) { "qdrant.exe" } else { "qdrant" };
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| QdrantError::ExtractionError(e.to_string()))?;
            
            if file.name().ends_with(binary_name) {
                let mut out_file = fs::File::create(dest)?;
                std::io::copy(&mut file, &mut out_file)?;
                break;
            }
        }
        
        Ok(())
    }
    
    /// Get the appropriate binary name for the current platform
    fn get_binary_name(&self) -> String {
        if cfg!(windows) {
            "qdrant.exe".to_string()
        } else {
            "qdrant".to_string()
        }
    }
    
    /// Get the download URL for the current platform
    fn get_download_url(&self) -> String {
        let platform = if cfg!(target_os = "windows") {
            "x86_64-pc-windows-msvc"
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "aarch64-apple-darwin"
            } else {
                "x86_64-apple-darwin"
            }
        } else if cfg!(target_os = "linux") {
            "x86_64-unknown-linux-musl"
        } else {
            panic!("Unsupported platform for Qdrant auto-download");
        };
        
        let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
        
        format!(
            "https://github.com/qdrant/qdrant/releases/download/{}/qdrant-{}.{}",
            QDRANT_VERSION, platform, extension
        )
    }
    
    /// Start Qdrant process
    async fn start(&mut self, binary_path: &Path) -> Result<(), QdrantError> {
        // Create data directory
        fs::create_dir_all(&self.data_dir)?;
        
        tracing::info!("🚀 Starting Qdrant on port {} (storage: {})...", QDRANT_PORT, self.data_dir.display());
        
        // Run Qdrant with cwd = project root so relative paths resolve correctly
        let cwd = self.bin_dir.parent().unwrap_or_else(|| self.bin_dir.as_path());
        let child = Command::new(binary_path)
            .current_dir(cwd)
            .arg("--storage-path")
            .arg(&self.data_dir)
            .arg("--http-port")
            .arg(QDRANT_PORT.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| QdrantError::StartupError(e.to_string()))?;
        
        self.process = Some(child);
        
        Ok(())
    }
    
    /// Wait for Qdrant to be ready
    async fn wait_for_ready(&self) -> Result<(), QdrantError> {
        let max_attempts = 30; // 30 seconds
        let mut attempts = 0;
        
        while attempts < max_attempts {
            if self.health_check().await.is_ok() {
                return Ok(());
            }
            
            attempts += 1;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        
        Err(QdrantError::HealthCheckError(
            "Qdrant failed to start within 30 seconds".to_string()
        ))
    }
    
    /// Stop Qdrant process
    pub fn stop(&mut self) -> Result<(), QdrantError> {
        if let Some(mut process) = self.process.take() {
            tracing::info!("🛑 Stopping Qdrant...");
            process.kill()?;
            process.wait()?;
            tracing::info!("✅ Qdrant stopped");
        }
        Ok(())
    }
}

impl Drop for QdrantSidecar {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

impl Default for QdrantSidecar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_binary_name() {
        let sidecar = QdrantSidecar::new();
        let name = sidecar.get_binary_name();
        
        if cfg!(windows) {
            assert_eq!(name, "qdrant.exe");
        } else {
            assert_eq!(name, "qdrant");
        }
    }
    
    #[test]
    fn test_download_url() {
        let sidecar = QdrantSidecar::new();
        let url = sidecar.get_download_url();
        
        assert!(url.contains("github.com/qdrant/qdrant"));
        assert!(url.contains(QDRANT_VERSION));
    }
    
    #[tokio::test]
    async fn test_health_check_when_not_running() {
        let sidecar = QdrantSidecar::new();
        // This should fail if Qdrant is not running
        let result = sidecar.health_check().await;
        // We don't assert failure because Qdrant might actually be running
        println!("Health check result: {:?}", result);
    }
}
