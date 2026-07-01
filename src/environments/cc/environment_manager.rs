use crate::core::environment_manager::{DynEnvironment, EnvironmentManager, EnvironmentType};
use crate::core::session::SessionManager;
use crate::infrastructure::config::{CcEnvironment as ConfigCcEnvironment, Config};
use crate::infrastructure::shell::ScriptGenerator;
use crate::infrastructure::shell::ShellType;
use serde_json;
use std::collections::HashMap;

/// CC (Claude Code) 环境管理器
pub struct CcEnvironmentManager {
    environments: HashMap<String, ConfigCcEnvironment>,
}

impl Default for CcEnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl CcEnvironmentManager {
    /// 创建新的 CC 环境管理器
    pub fn new() -> Self {
        let mut manager = Self {
            environments: HashMap::new(),
        };

        // 从配置文件加载 CC 环境
        if let Err(e) = manager.load_from_config() {
            crate::cli::print::warn(&format!("Failed to load CC environments from config: {e}"));
        }

        manager
    }

    /// 从配置文件加载 CC 环境
    fn load_from_config(&mut self) -> Result<(), String> {
        let config = Config::load()?;

        self.environments.clear();
        for env in &config.cc_environments {
            let cc_env = ConfigCcEnvironment {
                name: env.name.clone(),
                provider: env.provider.clone(),
                api_key: env.api_key.clone(),
                base_url: env.base_url.clone(),
                sonnet_model: env.sonnet_model.clone(),
                opus_model: env.opus_model.clone(),
                haiku_model: env.haiku_model.clone(),
                description: env.description.clone(),
                api_timeout_ms: env.api_timeout_ms.clone(),
                extra_env: env.extra_env.clone(),
            };

            self.environments.insert(env.name.clone(), cc_env);
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl EnvironmentManager for CcEnvironmentManager {
    fn environment_type(&self) -> EnvironmentType {
        EnvironmentType::Cc
    }

    fn list(&self) -> Result<Vec<DynEnvironment>, String> {
        let mut result = Vec::new();
        for env in self.environments.values() {
            result.push(DynEnvironment {
                name: env.name.clone(),
                path: env.base_url.clone(),
                version: Some(env.sonnet_model.clone()),
                description: Some(env.description.clone()),
                is_active: env.is_active(),
            });
        }
        Ok(result)
    }

    fn get(&self, name: &str) -> Result<Option<DynEnvironment>, String> {
        if let Some(env) = self.environments.get(name) {
            Ok(Some(DynEnvironment {
                name: env.name.clone(),
                path: env.base_url.clone(),
                version: Some(env.sonnet_model.clone()),
                description: Some(env.description.clone()),
                is_active: env.is_active(),
            }))
        } else {
            Ok(None)
        }
    }

    fn add(&mut self, name: &str, config_str: &str) -> Result<(), String> {
        // Parse config as JSON
        let config: serde_json::Value =
            serde_json::from_str(config_str).map_err(|e| format!("Failed to parse config: {e}"))?;

        let provider = config
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("anthropic");

        let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .ok_or("Missing base_url in config")?;

        // Support both "model" (legacy) and "sonnet_model" (new)
        let sonnet_model = config
            .get("sonnet_model")
            .or_else(|| config.get("model"))
            .and_then(|v| v.as_str())
            .unwrap_or("claude-sonnet-4-5");

        let opus_model = config.get("opus_model").and_then(|v| v.as_str());
        let haiku_model = config.get("haiku_model").and_then(|v| v.as_str());

        let default_desc = format!("CC: {name} ({sonnet_model})");
        let description = config
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_desc);

        // Create CC environment
        let cc_environment = ConfigCcEnvironment {
            name: name.to_string(),
            provider: provider.to_string(),
            description: description.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            sonnet_model: sonnet_model.to_string(),
            opus_model: opus_model.map(String::from),
            haiku_model: haiku_model.map(String::from),
            api_timeout_ms: config
                .get("api_timeout_ms")
                .and_then(|v| v.as_str())
                .map(String::from),
            extra_env: std::collections::HashMap::new(),
        };

        // 持久化到配置文件
        let mut file_config = Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
        if let Some(existing) = file_config
            .cc_environments
            .iter_mut()
            .find(|env| env.name == name)
        {
            existing.provider = provider.to_string();
            existing.api_key = api_key.to_string();
            existing.base_url = base_url.to_string();
            existing.sonnet_model = sonnet_model.to_string();
            existing.description = description.to_string();
            existing.opus_model = opus_model.map(String::from);
            existing.haiku_model = haiku_model.map(String::from);
            existing.api_timeout_ms = config
                .get("api_timeout_ms")
                .and_then(|v| v.as_str())
                .map(String::from);
            // extra_env is preserved as-is; CLI add/update does not touch it
        } else {
            file_config.cc_environments.push(cc_environment.clone());
        }
        file_config
            .save()
            .map_err(|e| format!("Failed to save config: {e}"))?;

        self.environments.insert(name.to_string(), cc_environment);
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        if self.environments.remove(name).is_none() {
            return Err(format!("CC environment '{name}' not found"));
        }

        let mut config = Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
        let original_len = config.cc_environments.len();
        config.cc_environments.retain(|env| env.name != name);
        if config.cc_environments.len() == original_len {
            return Err(format!("CC environment '{name}' not found"));
        }

        config
            .save()
            .map_err(|e| format!("Failed to save config: {e}"))?;

        Ok(())
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, String> {
        let cc_env = self
            .environments
            .get(name)
            .ok_or_else(|| format!("CC environment '{name}' not found"))?;

        let shell_type =
            shell_type.unwrap_or_else(crate::infrastructure::shell::platform::detect_shell);

        // Create config for script generation
        let mut config = serde_json::json!({
            "api_key": cc_env.api_key,
            "base_url": cc_env.base_url,
            "sonnet_model": cc_env.sonnet_model,
        });

        // Add CC-specific environment variables using provider factory
        let provider = crate::environments::cc::provider::get_provider(&cc_env.provider);
        provider.setup_config(cc_env, &mut config);

        let generator = ScriptGenerator::new().map_err(|e| e.to_string())?;
        match generator.generate_switch_script(EnvironmentType::Cc, name, &config, Some(shell_type))
        {
            Ok(script) => Ok(script),
            Err(e) => Err(format!("Failed to generate script: {e}")),
        }
    }

    fn get_current(&self) -> Result<Option<String>, String> {
        // Session 优先
        if let Ok(session) = SessionManager::new() {
            if let Some(current) = session.get_current_environment(EnvironmentType::Cc) {
                return Ok(Some(current.clone()));
            }
        }

        // 兜底：检查环境变量
        if let (Ok(auth_token), Ok(base_url)) = (
            std::env::var("ANTHROPIC_AUTH_TOKEN"),
            std::env::var("ANTHROPIC_BASE_URL"),
        ) {
            for (name, cc_env) in &self.environments {
                let env_token = cc_env.resolve_env_var(&cc_env.api_key);
                let env_base_url = cc_env.resolve_env_var(&cc_env.base_url);

                if auth_token == env_token && base_url == env_base_url {
                    return Ok(Some(name.clone()));
                }
            }
        }

        Ok(None)
    }

    async fn scan(&self) -> Result<Vec<DynEnvironment>, String> {
        let mut result = Vec::new();

        // "Scan" for CC environments by checking Anthropic environment variables
        if let (Ok(auth_token), Ok(base_url)) = (
            std::env::var("ANTHROPIC_AUTH_TOKEN"),
            std::env::var("ANTHROPIC_BASE_URL"),
        ) {
            let cc_env = ConfigCcEnvironment {
                name: "cc-detected".to_string(),
                provider: "anthropic".to_string(),
                description: "Detected CC environment from system variables".to_string(),
                api_key: auth_token,
                base_url,
                sonnet_model: std::env::var("ANTHROPIC_DEFAULT_SONNET_MODEL")
                    .unwrap_or_else(|_| "claude-sonnet-4-5".to_string()),
                opus_model: None,
                haiku_model: None,
                api_timeout_ms: None,
                extra_env: std::collections::HashMap::new(),
            };
            result.push(DynEnvironment {
                name: cc_env.name.clone(),
                path: cc_env.base_url.clone(),
                version: Some(cc_env.sonnet_model.clone()),
                description: Some(cc_env.description.clone()),
                is_active: cc_env.is_active(),
            });
        }

        Ok(result)
    }

    fn set_current(&mut self, _name: &str) -> Result<(), String> {
        // This would set the current environment by updating environment variables
        // For now, this is a no-op - the actual switching is handled by use_env
        Ok(())
    }

    fn is_available(&self, name: &str) -> Result<bool, String> {
        Ok(self.environments.contains_key(name))
    }

    fn get_details(&self, name: &str) -> Result<Option<DynEnvironment>, String> {
        self.get(name)
    }
}

// 为 ConfigCcEnvironment 添加扩展方法
impl ConfigCcEnvironment {
    fn is_active(&self) -> bool {
        let provider = crate::environments::cc::provider::get_provider(&self.provider);
        provider.is_active()
    }

    pub fn resolve_env_var(&self, value: &str) -> String {
        if value.starts_with("${") && value.ends_with('}') {
            let var_name = &value[2..value.len() - 1];
            std::env::var(var_name).unwrap_or_else(|_| value.to_string())
        } else {
            value.to_string()
        }
    }
}
