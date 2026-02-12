//! KB-07 (Relations) — Sled-backed store for business leads and relations.
//! Bare metal, direct host filesystem. Founder: Jamey (Root Sovereign).

use std::path::Path;

const KB07_DEFAULT_PATH: &str = "./data/kbs/kb07_relations";

/// KB-07 Relations: logs new business leads for the Root Sovereign (Jamey).
pub struct KB07Relations {
    db: sled::Db,
}

impl KB07Relations {
    /// Open KB-07 at the given path (direct host filesystem; no Docker).
    pub fn open(path: Option<impl AsRef<Path>>) -> sled::Result<Self> {
        let p = path
            .map(|x| x.as_ref().to_path_buf())
            .unwrap_or_else(|| Path::new(KB07_DEFAULT_PATH).to_path_buf());
        let db = sled::open(p)?;
        Ok(Self { db })
    }

    /// Log a new business lead. Key: lead id or timestamp; value: JSON or structured payload.
    pub fn log_lead(&self, key: &str, payload: &[u8]) -> sled::Result<Option<sled::IVec>> {
        self.db.insert(key.as_bytes(), payload)
    }

    /// List all lead keys (for audit; Root Sovereign).
    pub fn list_lead_keys(&self) -> sled::Result<Vec<Vec<u8>>> {
        Ok(self
            .db
            .iter()
            .keys()
            .filter_map(|k| k.ok())
            .map(|k| k.to_vec())
            .collect())
    }

    /// Get a single lead by key.
    pub fn get_lead(&self, key: &str) -> sled::Result<Option<Vec<u8>>> {
        self.db.get(key.as_bytes()).map(|o| o.map(|v| v.to_vec()))
    }

    /// Recent lead history for Scout context (last N entries by key order).
    pub fn recent_lead_history(&self, limit: usize) -> sled::Result<Vec<(String, Vec<u8>)>> {
        let mut pairs: Vec<(Vec<u8>, Vec<u8>)> = self
            .db
            .iter()
            .filter_map(|r| r.ok())
            .map(|(k, v)| (k.to_vec(), v.to_vec()))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs.reverse();
        pairs.truncate(limit);
        Ok(pairs
            .into_iter()
            .filter_map(|(k, v)| String::from_utf8(k).ok().map(|s| (s, v)))
            .collect())
    }

    /// Partners: keys with prefix "partner_" (value = JSON: webhook_url, email, city, etc.).
    pub fn get_partners(&self) -> sled::Result<Vec<(String, Vec<u8>)>> {
        let prefix: &[u8] = b"partner_";
        Ok(self
            .db
            .scan_prefix(prefix)
            .filter_map(|r| r.ok())
            .map(|(k, v)| (k.to_vec(), v.to_vec()))
            .filter_map(|(k, v)| String::from_utf8(k).ok().map(|s| (s, v)))
            .collect())
    }

    /// Log a high-intent alert for Command Center (Jamey). Key = "alert_{timestamp}".
    pub fn log_alert(&self, key: &str, payload: &[u8]) -> sled::Result<Option<sled::IVec>> {
        self.db.insert(key.as_bytes(), payload)
    }

    /// Register a new partner in KB-07. Key = "partner_{uuid}"; value = JSON.
    /// Returns the partner key (e.g. partner_123...) for reference.
    pub fn register_partner(
        &self,
        business_name: &str,
        primary_city: &str,
        service_type: &str,
        webhook_url: &str,
    ) -> sled::Result<String> {
        let uuid = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let key = format!("partner_{}", uuid);
        let payload = serde_json::json!({
            "business_name": business_name,
            "primary_city": primary_city,
            "service_type": service_type,
            "webhook_url": webhook_url,
        });
        self.db.insert(key.as_bytes(), payload.to_string().as_bytes())?;
        Ok(key)
    }
}
