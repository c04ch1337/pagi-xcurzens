//! Phoenix Auto-Update System
//!
//! Checks for new releases on GitHub and provides update functionality.
//! For private repos, requires a GitHub token with read access to releases.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const GITHUB_API_BASE: &str = "https://api.github.com";
const REPO_OWNER: &str = "your-github-username"; // TODO: Update with actual repo owner
const REPO_NAME: &str = "pagi-uac-main"; // TODO: Update with actual repo name

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub current: String,
    pub latest: Option<String>,
    pub update_available: bool,
    pub download_url: Option<String>,
}

/// GitHub Release API response
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    prerelease: bool,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Update checker
pub struct UpdateChecker {
    client: reqwest::Client,
    github_token: Option<String>,
}

impl UpdateChecker {
    /// Create a new update checker
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Phoenix-UAC")
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        
        // Try to get GitHub token from environment
        let github_token = std::env::var("GITHUB_TOKEN").ok()
            .or_else(|| std::env::var("PHOENIX_GITHUB_TOKEN").ok());
        
        Self {
            client,
            github_token,
        }
    }
    
    /// Get current version from VERSION file
    pub fn get_current_version() -> Result<String, Box<dyn std::error::Error>> {
        let version_path = Path::new("VERSION");
        if version_path.exists() {
            let version = fs::read_to_string(version_path)?;
            Ok(version.trim().to_string())
        } else {
            Ok("0.0.0".to_string())
        }
    }
    
    /// Check for updates
    pub async fn check_for_updates(&self) -> Result<VersionInfo, Box<dyn std::error::Error>> {
        let current = Self::get_current_version()?;
        
        // Get latest release from GitHub
        let url = format!("{}/repos/{}/{}/releases/latest", GITHUB_API_BASE, REPO_OWNER, REPO_NAME);
        
        let mut request = self.client.get(&url);
        
        // Add authorization header if token is available
        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Ok(VersionInfo {
                current: current.clone(),
                latest: None,
                update_available: false,
                download_url: None,
            });
        }
        
        let release: GitHubRelease = response.json().await?;
        
        // Remove 'v' prefix if present
        let latest = release.tag_name.trim_start_matches('v').to_string();
        
        // Compare versions
        let update_available = Self::is_newer_version(&current, &latest);
        
        // Find appropriate download URL for current platform
        let download_url = Self::get_platform_download_url(&release.assets);
        
        Ok(VersionInfo {
            current,
            latest: Some(latest),
            update_available,
            download_url,
        })
    }
    
    /// Compare two version strings (simple semantic versioning)
    fn is_newer_version(current: &str, latest: &str) -> bool {
        // Compare numeric semver core (major.minor.patch) first.
        let current_base = current.split('-').next().unwrap_or(current);
        let latest_base = latest.split('-').next().unwrap_or(latest);

        let current_parts: Vec<&str> = current_base.split('.').collect();
        let latest_parts: Vec<&str> = latest_base.split('.').collect();

        for i in 0..3 {
            let current_num: u32 = current_parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
            let latest_num: u32 = latest_parts.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);

            if latest_num > current_num {
                return true;
            } else if latest_num < current_num {
                return false;
            }
        }

        // Same numeric version: stable release is newer than any prerelease of the same version.
        // Example: current=0.1.0-beta.1, latest=0.1.0 => update available.
        let current_is_prerelease = current.contains('-');
        let latest_is_prerelease = latest.contains('-');
        if current_is_prerelease && !latest_is_prerelease {
            return true;
        }

        // Otherwise (both stable or latest is prerelease of same base), treat as not newer.
        false
    }
    
    /// Get download URL for current platform
    fn get_platform_download_url(assets: &[GitHubAsset]) -> Option<String> {
        let platform = if cfg!(target_os = "windows") {
            "windows-x86_64.zip"
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                "macos-aarch64.tar.gz"
            } else {
                "macos-x86_64.tar.gz"
            }
        } else if cfg!(target_os = "linux") {
            "linux-x86_64.tar.gz"
        } else {
            return None;
        };
        
        assets.iter()
            .find(|asset| asset.name.contains(platform))
            .map(|asset| asset.browser_download_url.clone())
    }
    
    /// Download update to a temporary file
    pub async fn download_update(&self, url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut request = self.client.get(url);
        
        // Add authorization header if token is available
        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }
        
        let response = request.send().await?;
        
        if !response.status().is_success() {
            return Err(format!("Failed to download update: {}", response.status()).into());
        }
        
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

impl Default for UpdateChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_comparison() {
        assert!(UpdateChecker::is_newer_version("0.1.0", "0.2.0"));
        assert!(UpdateChecker::is_newer_version("0.1.0", "1.0.0"));
        assert!(UpdateChecker::is_newer_version("0.1.0-beta.1", "0.1.0"));
        assert!(!UpdateChecker::is_newer_version("0.2.0", "0.1.0"));
        assert!(!UpdateChecker::is_newer_version("1.0.0", "0.9.9"));
    }
    
    #[test]
    fn test_get_current_version() {
        // This test will only pass if VERSION file exists
        if let Ok(version) = UpdateChecker::get_current_version() {
            assert!(!version.is_empty());
        }
    }
}
