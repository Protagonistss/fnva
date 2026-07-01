use crate::core::environment_manager::{DynEnvironment, EnvironmentManager, EnvironmentType};
use crate::core::session::SessionManager;
use crate::infrastructure::config::{Config, EnvironmentSource, MavenEnvironment};
use crate::infrastructure::shell::platform::detect_shell;
use crate::infrastructure::shell::{ScriptGenerator, ShellType};
use crate::utils::path::normalize_path;
use std::collections::HashMap;

use super::validator::validate_maven_home;

/// Maven 环境管理器(简化版:不做系统扫描,只从配置加载)。
pub struct MavenEnvironmentManager {
    installations: HashMap<String, MavenEnvironment>,
}

impl Default for MavenEnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MavenEnvironmentManager {
    pub fn new() -> Self {
        let mut manager = Self {
            installations: HashMap::new(),
        };
        if let Err(e) = manager.load_from_config() {
            crate::cli::print::warn(&format!(
                "Failed to load Maven environments from config: {e}"
            ));
        }
        manager
    }

    fn load_from_config(&mut self) -> Result<(), String> {
        let config = Config::load()?;
        self.installations.clear();
        for env in &config.maven_environments {
            self.installations.insert(env.name.clone(), env.clone());
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl EnvironmentManager for MavenEnvironmentManager {
    fn environment_type(&self) -> EnvironmentType {
        EnvironmentType::Maven
    }

    fn list(&self) -> Result<Vec<DynEnvironment>, String> {
        let current_env = self.get_current().ok().flatten();
        let result = self
            .installations
            .values()
            .map(|env| {
                let is_active = current_env.as_ref() == Some(&env.name);
                DynEnvironment {
                    name: env.name.clone(),
                    path: env.maven_home.clone(),
                    version: None,
                    description: Some(env.description.clone()),
                    is_active,
                }
            })
            .collect();
        Ok(result)
    }

    fn get(&self, name: &str) -> Result<Option<DynEnvironment>, String> {
        Ok(self.installations.get(name).map(|env| DynEnvironment {
            name: env.name.clone(),
            path: env.maven_home.clone(),
            version: None,
            description: Some(env.description.clone()),
            is_active: false,
        }))
    }

    fn add(&mut self, name: &str, config_str: &str) -> Result<(), String> {
        let cfg: serde_json::Value =
            serde_json::from_str(config_str).map_err(|e| format!("Failed to parse config: {e}"))?;
        let maven_home = cfg
            .get("maven_home")
            .and_then(|v| v.as_str())
            .ok_or("Missing maven_home in config")?;
        if !validate_maven_home(maven_home) {
            return Err("Invalid Maven installation".to_string());
        }
        let maven_opts = cfg
            .get("maven_opts")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let local_repo = cfg
            .get("local_repo")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let settings_file = cfg
            .get("settings_file")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let mut config = Config::load()?;
        config.add_maven_env(MavenEnvironment {
            name: name.to_string(),
            maven_home: maven_home.to_string(),
            description: format!("Maven ({maven_home})"),
            source: EnvironmentSource::Manual,
            maven_opts,
            local_repo,
            settings_file,
        })?;
        config.save()?;
        self.load_from_config()?;
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        let mut config = Config::load()?;
        config.remove_maven_env(name)?;
        if config.default_maven_env.as_deref() == Some(name) {
            config.default_maven_env = None;
        }
        if config.current_maven_env.as_deref() == Some(name) {
            config.current_maven_env = None;
        }
        config.save()?;
        self.installations.remove(name);
        Ok(())
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, String> {
        let env = self
            .installations
            .get(name)
            .ok_or_else(|| format!("Maven environment '{name}' not found"))?;
        if !validate_maven_home(&env.maven_home) {
            return Err(format!("Invalid MAVEN_HOME: {}", env.maven_home));
        }
        let shell_type = shell_type.unwrap_or_else(detect_shell);
        let config = serde_json::json!({
            "maven_home": env.maven_home,
            "maven_opts": env.maven_opts,
            "local_repo": env.local_repo,
            "settings_file": env.settings_file,
        });
        let generator = ScriptGenerator::new().map_err(|e| e.to_string())?;
        generator
            .generate_switch_script(EnvironmentType::Maven, name, &config, Some(shell_type))
            .map_err(|e| format!("Failed to generate script: {e}"))
    }

    fn get_current(&self) -> Result<Option<String>, String> {
        if let Ok(session) = SessionManager::new() {
            if let Some(current) = session.get_current_environment(EnvironmentType::Maven) {
                return Ok(Some(current.clone()));
            }
        }
        if let Ok(maven_home) = std::env::var("MAVEN_HOME") {
            let normalized = normalize_path(&maven_home);
            for (name, env) in &self.installations {
                if normalize_path(&env.maven_home) == normalized {
                    return Ok(Some(name.clone()));
                }
            }
        }
        Ok(None)
    }

    async fn scan(&self) -> Result<Vec<DynEnvironment>, String> {
        // Maven 不做系统扫描,返回当前已配置的环境
        self.list()
    }

    fn set_current(&mut self, _name: &str) -> Result<(), String> {
        Ok(())
    }

    fn is_available(&self, name: &str) -> Result<bool, String> {
        Ok(self.installations.contains_key(name))
    }

    fn get_details(&self, name: &str) -> Result<Option<DynEnvironment>, String> {
        self.get(name)
    }
}

impl MavenEnvironmentManager {
    /// 修改已有 Maven 环境的可选变量配置。
    /// 传 `Some("")` 表示清除该字段，`None` 表示保持不变。
    pub fn set_env_vars(
        &mut self,
        name: &str,
        maven_opts: Option<Option<String>>,
        local_repo: Option<Option<String>>,
        settings_file: Option<Option<String>>,
    ) -> Result<(), String> {
        let mut config = Config::load()?;
        let env = config
            .maven_environments
            .iter_mut()
            .find(|e| e.name == name)
            .ok_or_else(|| format!("Maven environment '{name}' not found"))?;

        if let Some(v) = maven_opts {
            env.maven_opts = v.filter(|s| !s.is_empty());
        }
        if let Some(v) = local_repo {
            env.local_repo = v.filter(|s| !s.is_empty());
        }
        if let Some(v) = settings_file {
            env.settings_file = v.filter(|s| !s.is_empty());
        }
        config.save()?;
        self.load_from_config()?;
        Ok(())
    }

    /// 以可读格式输出某个 Maven 环境的完整配置。
    pub fn show_env(&self, name: &str) -> Result<String, String> {
        let config = Config::load()?;
        let env = config
            .maven_environments
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| format!("Maven environment '{name}' not found"))?;

        let mut lines = vec![
            format!("Name        : {}", env.name),
            format!("MAVEN_HOME  : {}", env.maven_home),
            format!("Description : {}", env.description),
        ];
        match &env.maven_opts {
            Some(v) => lines.push(format!("MAVEN_OPTS  : {v}")),
            None => lines.push("MAVEN_OPTS  : (not set)".to_string()),
        }
        match &env.local_repo {
            Some(v) => lines.push(format!("local_repo  : {v}")),
            None => lines.push("local_repo  : (not set, uses ~/.m2/repository)".to_string()),
        }
        match &env.settings_file {
            Some(v) => lines.push(format!("settings    : {v}")),
            None => lines.push("settings    : (not set, uses default)".to_string()),
        }
        Ok(lines.join("\n"))
    }
}
