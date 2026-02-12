//! Compiler: write Rust code to a directory, run `cargo build --release`, return path to artifact.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::io;
use tracing::info;

use crate::SkillError;

/// Compiles Rust code (or a path to a crate) into a cdylib and returns the path to the built library.
///
/// - `compile_from_string`: writes `code` as `src/lib.rs` in a temp crate, builds, copies artifact to `output_path` (or default `./data/pagi_evolution/`).
/// - `compile_from_path`: runs `cargo build --release` in the given directory (must be a crate root).
pub struct Compiler;

impl Compiler {
    /// Compile a string of Rust code as the entire `src/lib.rs` of a cdylib crate.
    /// Copies the built .so/.dll to `output_path` so it remains valid after the temp dir is dropped.
    /// If `output_path` is None, uses `./data/pagi_evolution/<name>.so` (or .dll on Windows).
    pub fn compile_from_string(
        code: &str,
        name: &str,
        output_path: Option<PathBuf>,
    ) -> Result<PathBuf, SkillError> {
        let dir = tempfile::tempdir().map_err(|e| SkillError::Load(e.to_string()))?;
        let root = dir.path().to_path_buf();

        let toml = r#"
[package]
name = "pagi_dynamic_skill"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
libc = "0.2"
"#;
        std::fs::write(root.join("Cargo.toml"), toml)
            .map_err(|e| SkillError::Load(e.to_string()))?;
        std::fs::create_dir_all(root.join("src")).map_err(|e| SkillError::Load(e.to_string()))?;
        std::fs::write(root.join("src").join("lib.rs"), code)
            .map_err(|e| SkillError::Load(e.to_string()))?;

        let built = Self::build_and_return_lib_path(&root)?;
        let dest = output_path.unwrap_or_else(|| {
            let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let dir = base.join("data").join("pagi_evolution");
            let _ = std::fs::create_dir_all(&dir);
            let ext = if cfg!(target_os = "windows") { "dll" } else { "so" };
            dir.join(format!("{}.{}", name, ext))
        });
        std::fs::copy(&built, &dest).map_err(|e: io::Error| SkillError::Load(e.to_string()))?;
        Ok(dest)
    }

    /// Compile a crate at the given path (directory containing Cargo.toml).
    /// The crate must be a cdylib and export the required C symbols.
    pub fn compile_from_path<P: AsRef<Path>>(crate_root: P) -> Result<PathBuf, SkillError> {
        let root = crate_root.as_ref().to_path_buf();
        if !root.join("Cargo.toml").exists() {
            return Err(SkillError::Load(format!(
                "not a crate root (no Cargo.toml): {}",
                root.display()
            )));
        }
        Self::build_and_return_lib_path(&root)
    }

    /// Run cargo build --release and return the path to the built cdylib.
    fn build_and_return_lib_path(root: &Path) -> Result<PathBuf, SkillError> {
        let target_dir = root.join("target");
        let status = Command::new("cargo")
            .current_dir(root)
            .args(["build", "--release", "--target-dir", target_dir.as_os_str().to_str().unwrap()])
            .status()
            .map_err(|e| SkillError::Load(format!("cargo build spawn failed: {}", e)))?;

        if !status.success() {
            return Err(SkillError::Load("cargo build failed".to_string()));
        }

        // target/release/libpagi_dynamic_skill.so (Unix) or pagi_dynamic_skill.dll (Windows)
        let lib_name = if cfg!(target_os = "windows") {
            "pagi_dynamic_skill.dll"
        } else {
            "libpagi_dynamic_skill.so"
        };
        let lib_path = target_dir.join("release").join(lib_name);
        if lib_path.exists() {
            info!(
                target: "pagi::evolution",
                path = %lib_path.display(),
                "Compiled dynamic skill library"
            );
            Ok(lib_path)
        } else {
            // Try with "lib" prefix on Unix (crate name may be normalized)
            let alt = target_dir.join("release").join("libpagi_dynamic_skill.so");
            if alt.exists() {
                Ok(alt)
            } else {
                Err(SkillError::Load(format!(
                    "artifact not found at {} or {}",
                    lib_path.display(),
                    alt.display()
                )))
            }
        }
    }
}
