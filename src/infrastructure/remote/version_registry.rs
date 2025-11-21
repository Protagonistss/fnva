use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub version: String,
    pub major: u32,
    #[serde(default)]
    pub lts: bool,
    pub tag_name: String,
    #[serde(default)]
    pub assets: HashMap<String, String>,
    #[serde(default)]
    pub assets_github: Option<HashMap<String, String>>,
    #[serde(default)]
    pub assets_aliyun: Option<HashMap<String, String>>,
    #[serde(default)]
    pub assets_tsinghua: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRegistry {
    pub versions: Vec<RegistryEntry>,
}

impl VersionRegistry {
    pub fn load() -> Result<Self, String> {
        // 1. Config explicit path
        if let Ok(cfg) = crate::infrastructure::config::Config::load() {
            if let Some(path) = cfg.java_download_sources.java_versions_path.as_ref() {
                if let Ok(Some(reg)) = try_read_toml(Ok(PathBuf::from(path))) { return Ok(reg); }
            }
        }

        // 2. Environment variable
        if let Ok(path) = std::env::var("FNVA_JAVA_VERSIONS_PATH") {
            if let Ok(Some(reg)) = try_read_toml(Ok(PathBuf::from(path))) {
                return Ok(reg);
            }
        }

        // 3. User home
        if let Some(p) = dirs::home_dir().map(|d| d.join(".fnva").join("java_versions.toml")) {
            if let Ok(Some(reg)) = try_read_toml(Ok(p)) { return Ok(reg); }
        }

        // 4. Executable dir config
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let p = dir.join("config").join("java_versions.toml");
                if let Ok(Some(reg)) = try_read_toml(Ok(p)) { return Ok(reg); }
            }
        }

        // 5. Embedded fallback
        const EMBEDDED: &str = include_str!("../../../config/java_versions.toml");
        if !EMBEDDED.trim().is_empty() {
            let reg: VersionRegistry = toml::from_str(EMBEDDED).map_err(|e| e.to_string())?;
            return Ok(reg);
        }

        Err("registry not found".to_string())
    }

    pub fn list(&self) -> Vec<RegistryEntry> {
        self.versions.clone()
    }

    pub fn find(&self, spec: &str) -> Option<RegistryEntry> {
        let cleaned = spec.trim().to_lowercase().replace("v", "").replace("jdk", "").replace("java", "");
        if cleaned == "lts" || cleaned == "latest-lts" {
            let mut lts: Vec<&RegistryEntry> = self.versions.iter().filter(|v| v.lts).collect();
            lts.sort_by(|a, b| b.major.cmp(&a.major));
            return lts.first().cloned().cloned();
        }
        if cleaned == "latest" || cleaned == "newest" {
            let mut all: Vec<&RegistryEntry> = self.versions.iter().collect();
            all.sort_by(|a, b| b.major.cmp(&a.major));
            return all.first().cloned().cloned();
        }
        if let Ok(m) = cleaned.parse::<u32>() {
            let mut same: Vec<&RegistryEntry> = self.versions.iter().filter(|v| v.major == m).collect();
            same.sort_by(|a, b| b.version.cmp(&a.version));
            return same.first().cloned().cloned();
        }
        for v in &self.versions {
            if v.version.to_lowercase().starts_with(&cleaned) || v.tag_name.to_lowercase().contains(&cleaned) {
                return Some(v.clone());
            }
        }
        None
    }
}

fn try_read_toml(path: Result<PathBuf, std::io::Error>) -> Result<Option<VersionRegistry>, String> {
    match path {
        Ok(p) => {
            if p.exists() {
                let s = fs::read_to_string(&p).map_err(|e| e.to_string())?;
                let reg: VersionRegistry = toml::from_str(&s).map_err(|e| e.to_string())?;
                Ok(Some(reg))
            } else {
                Ok(None)
            }
        }
        Err(_) => Ok(None),
    }
}

pub fn split_version(version: &str) -> (Option<u32>, Option<u32>) {
    let parts: Vec<&str> = version.split('.').collect();
    let minor = parts.get(1).and_then(|s| s.parse::<u32>().ok());
    let patch = parts.get(2).and_then(|s| s.parse::<u32>().ok());
    (minor, patch)
}
