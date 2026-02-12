//! SAO Redaction Filter: replace protected terms in meeting transcripts with [PROTECTED_TERM].
//!
//! Loads a list of Protected Terms from a local file (e.g. `data/protected_terms.txt`).
//! Optional project-level `.sao_policy` can add terms. Used before sending transcript to the
//! MinuteGenEngine or writing the final Markdown summary so the model and stored output
//! only see sanitized text (unless Sovereign Override is enabled).

use regex::Regex;
use std::path::Path;
use std::fs;
use std::io::BufRead;

/// Placeholder used in place of any protected term in sanitized output.
pub const PROTECTED_PLACEHOLDER: &str = "[PROTECTED_TERM]";

/// Loads protected terms from files and sanitizes text via regex replacement.
#[derive(Debug, Default)]
pub struct SAORedactor {
    terms: Vec<String>,
    /// Compiled regex that matches any of the protected terms (case-insensitive, word boundaries).
    pattern: Option<Regex>,
}

impl SAORedactor {
    /// Create an empty redactor (no terms; sanitize is a no-op).
    pub fn empty() -> Self {
        Self {
            terms: Vec::new(),
            pattern: None,
        }
    }

    /// Load protected terms from a file (one term per line; empty lines and lines starting with # ignored).
    pub fn load_from_path(path: &Path) -> std::io::Result<Self> {
        let terms = Self::read_terms_from_file(path)?;
        Ok(Self::from_terms(terms))
    }

    /// Load from the default path: `data/protected_terms.txt` (relative to `data_dir`).
    pub fn load_from_data_dir(data_dir: &Path) -> std::io::Result<Self> {
        let path = data_dir.join("protected_terms.txt");
        Self::load_from_path(&path)
    }

    /// Load global terms from `data_dir/protected_terms.txt`, then merge project-specific terms from
    /// `project_path/.sao_policy` if `project_path` is provided and that file exists.
    /// Use this at runtime when an active project is known so redaction respects both global and project policy.
    pub fn load_global_then_merge_project(
        data_dir: &Path,
        project_path: Option<&Path>,
    ) -> std::io::Result<Self> {
        let mut redactor = Self::load_from_data_dir(data_dir)?;
        if let Some(proj) = project_path {
            let policy = proj.join(".sao_policy");
            let _ = redactor.merge_terms_from_path(&policy);
        }
        Ok(redactor)
    }

    /// Merge additional terms from a second file (e.g. project `.sao_policy`). Preserves existing terms.
    pub fn merge_terms_from_path(&mut self, path: &Path) -> std::io::Result<()> {
        let extra = Self::read_terms_from_file(path)?;
        self.terms.extend(extra);
        self.rebuild_pattern();
        Ok(())
    }

    /// Build redactor from a list of terms. Terms are escaped for regex and combined with |.
    pub fn from_terms(terms: Vec<String>) -> Self {
        let mut s = Self::empty();
        s.set_terms(terms);
        s
    }

    fn set_terms(&mut self, terms: Vec<String>) {
        self.terms = terms
            .into_iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        self.rebuild_pattern();
    }

    fn rebuild_pattern(&mut self) {
        if self.terms.is_empty() {
            self.pattern = None;
            return;
        }
        let escaped: Vec<String> = self.terms.iter().map(|t| regex::escape(t)).collect();
        let alternation = escaped.join("|");
        let pattern_str = format!(r"(?i)\b(?:{})\b", alternation);
        self.pattern = Regex::new(&pattern_str).ok();
    }

    fn read_terms_from_file(path: &Path) -> std::io::Result<Vec<String>> {
        Self::read_terms_from_path(path)
    }

    /// Read protected terms from a file (one per line; empty and # lines ignored). Public for API that returns global vs local lists.
    pub fn read_terms_from_path(path: &Path) -> std::io::Result<Vec<String>> {
        if !path.exists() {
            return Ok(Vec::new());
        }
        let f = fs::File::open(path)?;
        let mut terms = Vec::new();
        for line in std::io::BufReader::new(f).lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            terms.push(line.to_string());
        }
        Ok(terms)
    }

    /// Sanitize transcript text: replace any protected term with [PROTECTED_TERM].
    pub fn sanitize_transcript(&self, text: String) -> String {
        match &self.pattern {
            None => text,
            Some(r) => r.replace_all(&text, PROTECTED_PLACEHOLDER).to_string(),
        }
    }

    /// Return true if any protected terms are loaded.
    pub fn is_active(&self) -> bool {
        self.pattern.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_redactor_returns_unchanged() {
        let r = SAORedactor::empty();
        assert_eq!(r.sanitize_transcript("The VANGUARD initiative.".to_string()), "The VANGUARD initiative.");
    }

    #[test]
    fn single_term_redacted() {
        let r = SAORedactor::from_terms(vec!["VANGUARD".to_string()]);
        assert_eq!(
            r.sanitize_transcript("The VANGUARD initiative.".to_string()),
            "The [PROTECTED_TERM] initiative."
        );
    }

    #[test]
    fn case_insensitive() {
        let r = SAORedactor::from_terms(vec!["VANGUARD".to_string()]);
        assert_eq!(
            r.sanitize_transcript("The vanguard initiative.".to_string()),
            "The [PROTECTED_TERM] initiative."
        );
    }
}
