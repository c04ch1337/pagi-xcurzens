//! Sovereign Sync: safely sync `.env` with `.env.example` without overwriting existing keys.
//!
//! On gateway startup, this adds any *missing* keys from the example to the live `.env`,
//! preserving the user's API keys and existing values. If `.env` does not exist, it is
//! created by copying `.env.example`.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Extracts the env key from a line (trimmed part before the first `=`).
/// Returns `None` if the line is not a key=value line (e.g. comment or empty).
fn key_from_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let key = trimmed.split('=').next()?.trim();
    if key.is_empty() {
        return None;
    }
    Some(key.to_string())
}

/// Collects all keys currently defined in the live env file (non-comment lines with `=`).
fn keys_in_file(path: &Path) -> std::io::Result<HashSet<String>> {
    let content = fs::read_to_string(path)?;
    let mut set = HashSet::new();
    for line in content.lines() {
        if let Some(k) = key_from_line(line) {
            set.insert(k);
        }
    }
    Ok(set)
}

/// Syncs the live `.env` with `.env.example`: appends any missing keys from the example
/// without overwriting or deleting existing entries.
///
/// - If `live_path` does not exist, copies `example_path` to `live_path` and returns `Ok(Some(n))`
///   where `n` is the number of keys in the example (or `None` if copy failed to parse).
/// - Otherwise, reads both files, and appends each line from the example that defines a key
///   not present in the live file. Returns `Ok(added_count)`.
///
/// **Safety:** Never overwrites or removes existing keys in the live file.
pub fn sync_env_files(example_path: &str, live_path: &str) -> std::io::Result<u32> {
    let example_path = Path::new(example_path);
    let live_path = Path::new(live_path);

    if !example_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Example file not found: {}", example_path.display()),
        ));
    }

    if !live_path.exists() {
        fs::copy(example_path, live_path)?;
        let n = keys_in_file(live_path).ok().map(|s| s.len()).unwrap_or(0);
        tracing::info!(
            target: "pagi::env_sync",
            "Sovereign Sync: .env did not exist; created from .env.example ({} keys).",
            n
        );
        // Return number of keys created so callers/tests can verify sync coverage.
        return Ok(n as u32);
    }

    let live_keys = keys_in_file(live_path)?;
    let example_content = fs::read_to_string(example_path)?;
    let mut to_append = Vec::new();
    for line in example_content.lines() {
        if let Some(key) = key_from_line(line) {
            if !live_keys.contains(&key) {
                to_append.push(line.to_string());
            }
        }
    }

    if to_append.is_empty() {
        return Ok(0);
    }

    let mut f = fs::OpenOptions::new().append(true).open(live_path)?;
    for line in &to_append {
        writeln!(f, "{}", line)?;
    }
    f.sync_all()?;
    let added = to_append.len() as u32;
    tracing::info!(
        target: "pagi::env_sync",
        "Sovereign Sync: Added {} new configuration key(s) from .env.example.",
        added
    );
    Ok(added)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn key_from_line_parses_key() {
        assert_eq!(key_from_line("FOO=bar"), Some("FOO".to_string()));
        assert_eq!(key_from_line("  PAGI_X = y  "), Some("PAGI_X".to_string()));
        assert_eq!(key_from_line("# FOO=bar"), None);
        assert_eq!(key_from_line(""), None);
        assert_eq!(key_from_line("   "), None);
    }

    #[test]
    fn sync_creates_env_if_missing() {
        let tmp = std::env::temp_dir();
        let example = tmp.join("env_sync_example");
        let live = tmp.join("env_sync_live_missing");
        let _ = fs::remove_file(&live);
        fs::write(&example, "A=1\nB=2\n").unwrap();
        let added = sync_env_files(example.to_str().unwrap(), live.to_str().unwrap()).unwrap();
        assert!(live.exists());
        let content = fs::read_to_string(&live).unwrap();
        assert!(content.contains("A=1"));
        assert!(content.contains("B=2"));
        assert!(added >= 2);
        let _ = fs::remove_file(&live);
        let _ = fs::remove_file(&example);
    }

    #[test]
    fn sync_does_not_overwrite() {
        let tmp = std::env::temp_dir();
        let example = tmp.join("env_sync_ex2");
        let live = tmp.join("env_sync_live2");
        fs::write(&example, "A=1\nB=from_example\n").unwrap();
        fs::write(&live, "B=user_kept\n").unwrap();
        let added = sync_env_files(example.to_str().unwrap(), live.to_str().unwrap()).unwrap();
        assert_eq!(added, 1);
        let content = fs::read_to_string(&live).unwrap();
        assert!(content.contains("B=user_kept"));
        assert!(content.contains("A=1"));
        let _ = fs::remove_file(&live);
        let _ = fs::remove_file(&example);
    }
}
