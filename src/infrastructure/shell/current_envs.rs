use crate::core::environment_manager::EnvironmentType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Persistent per-type record of the active environment.
/// Stored at `~/.fnva/current_envs.toml`.
///
/// Format:
/// ```toml
/// cc = "glmcc"
/// java = "jdk21"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrentEnvsFile {
    #[serde(default)]
    pub cc: Option<String>,
    #[serde(default)]
    pub java: Option<String>,
    #[serde(default)]
    pub llm: Option<String>,
}

impl CurrentEnvsFile {
    fn path() -> Result<PathBuf, String> {
        let home = dirs::home_dir().ok_or("Cannot get home directory")?;
        Ok(home.join(".fnva").join("current_envs.toml"))
    }

    /// Read the file. Returns default (all None) if missing.
    pub fn read() -> Result<Self, String> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read current_envs.toml: {e}"))?;
        toml::from_str(&content)
            .map_err(|e| format!("Failed to parse current_envs.toml: {e}"))
    }

    /// Write (or update) the entry for one environment type.
    pub fn write(env_type: EnvironmentType, name: &str) -> Result<(), String> {
        let mut file = Self::read()?;
        file.set(env_type, Some(name.to_string()));
        file.save()
    }

    /// Clear the entry for one environment type.
    pub fn clear(env_type: EnvironmentType) -> Result<(), String> {
        let mut file = Self::read()?;
        file.set(env_type, None);
        file.save()
    }

    /// Set a field by EnvironmentType.
    fn set(&mut self, env_type: EnvironmentType, value: Option<String>) {
        match env_type {
            EnvironmentType::Cc => self.cc = value,
            EnvironmentType::Java => self.java = value,
            EnvironmentType::Llm => self.llm = value,
        }
    }

    /// Convert to a HashMap for template rendering.
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        if let Some(ref v) = self.cc { map.insert("cc".to_string(), v.clone()); }
        if let Some(ref v) = self.java { map.insert("java".to_string(), v.clone()); }
        if let Some(ref v) = self.llm { map.insert("llm".to_string(), v.clone()); }
        map
    }

    /// Persist to disk.
    fn save(&self) -> Result<(), String> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .fnva dir: {e}"))?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize current_envs: {e}"))?;
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write current_envs.toml: {e}"))?;
        Ok(())
    }
}
