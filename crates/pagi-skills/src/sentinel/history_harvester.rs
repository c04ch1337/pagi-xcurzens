//! HistoryHarvester: Sentinel capability for browser history extraction
//! 
//! This module reads browser history from local SQLite databases (Brave/Chrome)
//! and pipes them into KB-3 (Global History) for behavior analysis.
//! 
//! Handles file-locking issues by creating temporary copies of active databases.

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Browser types supported by the harvester
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserType {
    Chrome,
    Brave,
    Edge,
    Firefox,
}

/// A single browser history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelHistoryEntry {
    pub url: String,
    pub title: Option<String>,
    pub visit_count: i64,
    pub last_visit_time: i64,
    pub browser: BrowserType,
}

/// Result of a history harvest operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentinelHistoryHarvestResult {
    pub browser: BrowserType,
    pub entries_harvested: usize,
    pub success: bool,
    pub message: String,
}

/// HistoryHarvester action for extracting browser history
pub struct SentinelHistoryHarvesterAction {
    /// Temporary directory for database copies
    temp_dir: PathBuf,
}

impl SentinelHistoryHarvesterAction {
    /// Create a new HistoryHarvester action
    pub fn new() -> Result<Self, String> {
        let temp_dir = std::env::temp_dir().join("pagi_sentinel_history");
        fs::create_dir_all(&temp_dir).map_err(|e| format!("Failed to create temp dir: {}", e))?;
        
        Ok(Self { temp_dir })
    }

    /// Get the default history database path for a browser
    fn get_browser_history_path(&self, browser: &BrowserType) -> Option<PathBuf> {
        let home = dirs::home_dir()?;

        match browser {
            BrowserType::Chrome => {
                #[cfg(target_os = "windows")]
                {
                    Some(home.join("AppData/Local/Google/Chrome/User Data/Default/History"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(home.join("Library/Application Support/Google/Chrome/Default/History"))
                }
                #[cfg(target_os = "linux")]
                {
                    Some(home.join(".config/google-chrome/Default/History"))
                }
            }
            BrowserType::Brave => {
                #[cfg(target_os = "windows")]
                {
                    Some(home.join("AppData/Local/BraveSoftware/Brave-Browser/User Data/Default/History"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(home.join("Library/Application Support/BraveSoftware/Brave-Browser/Default/History"))
                }
                #[cfg(target_os = "linux")]
                {
                    Some(home.join(".config/BraveSoftware/Brave-Browser/Default/History"))
                }
            }
            BrowserType::Edge => {
                #[cfg(target_os = "windows")]
                {
                    Some(home.join("AppData/Local/Microsoft/Edge/User Data/Default/History"))
                }
                #[cfg(target_os = "macos")]
                {
                    Some(home.join("Library/Application Support/Microsoft Edge/Default/History"))
                }
                #[cfg(target_os = "linux")]
                {
                    Some(home.join(".config/microsoft-edge/Default/History"))
                }
            }
            BrowserType::Firefox => {
                // Firefox uses a different structure with profile directories
                warn!("Firefox history harvesting not yet implemented");
                None
            }
        }
    }

    /// Create a temporary copy of the database to avoid locking issues
    fn create_temp_copy(&self, source: &Path, browser: &BrowserType) -> Result<PathBuf, String> {
        let timestamp = chrono::Utc::now().timestamp();
        let browser_name = format!("{:?}", browser).to_lowercase();
        let temp_path = self.temp_dir.join(format!("{}_{}.db", browser_name, timestamp));

        debug!("[SENTINEL] Creating temp copy: {:?} -> {:?}", source, temp_path);

        fs::copy(source, &temp_path)
            .map_err(|e| format!("Failed to copy database: {}", e))?;

        Ok(temp_path)
    }

    /// Extract history entries from a browser database
    fn extract_history(
        &self,
        db_path: &Path,
        browser: &BrowserType,
        limit: usize,
    ) -> Result<Vec<SentinelHistoryEntry>, String> {
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .map_err(|e| format!("Failed to open database: {}", e))?;

        let mut stmt = conn
            .prepare(
                "SELECT url, title, visit_count, last_visit_time 
                 FROM urls 
                 ORDER BY last_visit_time DESC 
                 LIMIT ?1",
            )
            .map_err(|e| format!("Failed to prepare statement: {}", e))?;

        let entries = stmt
            .query_map([limit], |row| {
                Ok(SentinelHistoryEntry {
                    url: row.get(0)?,
                    title: row.get(1).ok(),
                    visit_count: row.get(2)?,
                    last_visit_time: row.get(3)?,
                    browser: browser.clone(),
                })
            })
            .map_err(|e| format!("Failed to query history: {}", e))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect results: {}", e))?;

        Ok(entries)
    }

    /// Harvest history from a specific browser
    pub fn harvest_browser(
        &self,
        browser: BrowserType,
        limit: usize,
    ) -> Result<SentinelHistoryHarvestResult, String> {
        info!("[SENTINEL] Harvesting history from {:?}", browser);

        // Get the browser history path
        let history_path = self
            .get_browser_history_path(&browser)
            .ok_or_else(|| format!("Could not determine history path for {:?}", browser))?;

        if !history_path.exists() {
            return Ok(SentinelHistoryHarvestResult {
                browser,
                entries_harvested: 0,
                success: false,
                message: format!("History file not found: {:?}", history_path),
            });
        }

        // Create a temporary copy to avoid locking issues
        let temp_path = match self.create_temp_copy(&history_path, &browser) {
            Ok(path) => path,
            Err(e) => {
                return Ok(SentinelHistoryHarvestResult {
                    browser,
                    entries_harvested: 0,
                    success: false,
                    message: format!("Failed to create temp copy: {}", e),
                });
            }
        };

        // Extract history from the temporary copy
        let entries = match self.extract_history(&temp_path, &browser, limit) {
            Ok(entries) => entries,
            Err(e) => {
                // Clean up temp file
                let _ = fs::remove_file(&temp_path);
                return Ok(SentinelHistoryHarvestResult {
                    browser,
                    entries_harvested: 0,
                    success: false,
                    message: format!("Failed to extract history: {}", e),
                });
            }
        };

        let count = entries.len();

        // Clean up temp file
        if let Err(e) = fs::remove_file(&temp_path) {
            warn!("[SENTINEL] Failed to clean up temp file: {}", e);
        }

        info!("[SENTINEL] Harvested {} entries from {:?}", count, browser);

        Ok(SentinelHistoryHarvestResult {
            browser,
            entries_harvested: count,
            success: true,
            message: format!("Successfully harvested {} entries", count),
        })
    }

    /// Harvest history from all supported browsers
    pub fn harvest_all(&self, limit_per_browser: usize) -> Vec<SentinelHistoryHarvestResult> {
        let browsers = vec![
            BrowserType::Chrome,
            BrowserType::Brave,
            BrowserType::Edge,
        ];

        browsers
            .into_iter()
            .filter_map(|browser| self.harvest_browser(browser, limit_per_browser).ok())
            .collect()
    }

    /// Extract history entries and return them for KB-3 ingestion
    pub fn harvest_for_kb3(
        &self,
        browser: BrowserType,
        limit: usize,
    ) -> Result<Vec<SentinelHistoryEntry>, String> {
        let history_path = self
            .get_browser_history_path(&browser)
            .ok_or_else(|| format!("Could not determine history path for {:?}", browser))?;

        if !history_path.exists() {
            return Err(format!("History file not found: {:?}", history_path));
        }

        let temp_path = self.create_temp_copy(&history_path, &browser)?;
        let entries = self.extract_history(&temp_path, &browser, limit)?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_path);

        Ok(entries)
    }
}

impl Default for SentinelHistoryHarvesterAction {
    fn default() -> Self {
        Self::new().expect("Failed to create HistoryHarvester")
    }
}

// Helper crate for home directory detection
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            std::env::var("USERPROFILE").ok().map(PathBuf::from)
        }
        #[cfg(not(target_os = "windows"))]
        {
            std::env::var("HOME").ok().map(PathBuf::from)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_type_serialization() {
        let browser = BrowserType::Brave;
        let json = serde_json::to_string(&browser).unwrap();
        let deserialized: BrowserType = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            BrowserType::Brave => (),
            _ => panic!("Deserialization failed"),
        }
    }

    #[test]
    fn test_history_entry_creation() {
        let entry = SentinelHistoryEntry {
            url: "https://example.com".to_string(),
            title: Some("Example".to_string()),
            visit_count: 5,
            last_visit_time: 1234567890,
            browser: BrowserType::Chrome,
        };

        assert_eq!(entry.url, "https://example.com");
        assert_eq!(entry.visit_count, 5);
    }
}
