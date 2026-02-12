//! SkillLoader: load .so/.dll via libloading and execute via C ABI.

use libloading::Library;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::skill::{DynamicSkill, SkillError};

/// C ABI: execute(args_json) returns allocated C string or null.
type ExecuteFn = unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_char;
/// C ABI: free string returned by execute.
type FreeFn = unsafe extern "C" fn(*mut std::ffi::c_char);

/// Wrapper that calls into a loaded library via C ABI.
struct LoadedSkill {
    _lib: Library,
    execute: ExecuteFn,
    free: FreeFn,
}

impl LoadedSkill {
    fn execute_json(&self, args: &Value) -> Result<Value, SkillError> {
        let args_str = serde_json::to_string(args).map_err(SkillError::Serialization)?;
        let c_args = std::ffi::CString::new(args_str.as_bytes())
            .map_err(|e| SkillError::Execution(e.to_string()))?;
        let out_ptr = unsafe { (self.execute)(c_args.as_ptr()) };
        if out_ptr.is_null() {
            return Err(SkillError::Execution("skill returned null".to_string()));
        }
        let out_str = unsafe {
            let s = std::ffi::CStr::from_ptr(out_ptr).to_string_lossy().into_owned();
            (self.free)(out_ptr);
            s
        };
        let value: Value = serde_json::from_str(&out_str).map_err(SkillError::Serialization)?;
        Ok(value)
    }
}

/// Adapter so LoadedSkill can be used as DynamicSkill (we need to pass args by reference).
struct LoadedSkillAdapter(Arc<LoadedSkill>);

impl DynamicSkill for LoadedSkillAdapter {
    fn execute(&self, args: Value) -> Result<Value, SkillError> {
        self.0.execute_json(&args)
    }
}

/// Loads dynamic libraries and dispatches execute by skill name.
/// Stores library handles so symbols remain valid; hot-reload = drop old, load new under same name.
pub struct SkillLoader {
    skills: RwLock<HashMap<String, Arc<LoadedSkillAdapter>>>,
}

impl SkillLoader {
    pub fn new() -> Self {
        Self {
            skills: RwLock::new(HashMap::new()),
        }
    }

    /// Load a dynamic library from the given path and register it under `name`.
    /// If `name` was already loaded, the previous library is replaced (hot-reload).
    pub fn load<P: AsRef<Path>>(&self, path: P, name: String) -> Result<(), SkillError> {
        let path = path.as_ref();
        let lib = unsafe {
            Library::new(path).map_err(|e| SkillError::Load(format!("libloading: {}", e)))?
        };
        let execute_fn = unsafe {
            *lib.get(b"pagi_dynamic_skill_execute")
                .map_err(|e| SkillError::Load(format!("symbol pagi_dynamic_skill_execute: {}", e)))?
        };
        let free_fn = unsafe {
            *lib.get(b"pagi_dynamic_skill_free")
                .map_err(|e| SkillError::Load(format!("symbol pagi_dynamic_skill_free: {}", e)))?
        };
        let loaded = LoadedSkill {
            _lib: lib,
            execute: execute_fn,
            free: free_fn,
        };
        let adapter = Arc::new(LoadedSkillAdapter(Arc::new(loaded)));
        self.skills.write().map_err(|e| SkillError::Load(e.to_string()))?.insert(name, adapter);
        Ok(())
    }

    /// Execute a loaded skill by name with the given JSON args.
    pub fn execute(&self, name: &str, args: Value) -> Result<Value, SkillError> {
        let guard = self.skills.read().map_err(|e| SkillError::Load(e.to_string()))?;
        let skill = guard.get(name).ok_or_else(|| SkillError::NotLoaded(name.to_string()))?;
        skill.execute(args)
    }

    /// Unload a skill by name (drops the library handle).
    pub fn unload(&self, name: &str) -> bool {
        self.skills.write().map(|mut g| g.remove(name).is_some()).unwrap_or(false)
    }

    /// List names of currently loaded dynamic skills.
    pub fn loaded_names(&self) -> Vec<String> {
        self.skills
            .read()
            .map(|g| g.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for SkillLoader {
    fn default() -> Self {
        Self::new()
    }
}
