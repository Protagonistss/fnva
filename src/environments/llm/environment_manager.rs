use crate::core::environment_manager::{DynEnvironment, EnvironmentManager, EnvironmentType};
use crate::core::session::SessionManager;
use crate::infrastructure::config::{Config, LlmEnvironment as ConfigLlmEnvironment};
use crate::infrastructure::shell::ScriptGenerator;
use crate::infrastructure::shell::ShellType;
use serde_json;
use std::collections::HashMap;

/// LLM 环境管理器
pub struct LlmEnvironmentManager {
    environments: HashMap<String, LlmEnvironment>,
}

impl LlmEnvironmentManager {
    /// 创建新的 LLM 环境管理器
    pub fn new() -> Self {
        let mut manager = Self {
            environments: HashMap::new(),
        };

        // 从配置文件加载 LLM 环境
        if let Err(e) = manager.load_from_config() {
            eprintln!(
                "Warning: Failed to load LLM environments from config: {}",
                e
            );
        }

        manager
    }

    /// 从配置文件加载 LLM 环境
    fn load_from_config(&mut self) -> Result<(), String> {
        let config = Config::load()?;

        self.environments.clear();
        for env in &config.llm_environments {
            let llm_env = LlmEnvironment {
                name: env.name.clone(),
                provider: env.provider.clone(),
                api_key: env.api_key.clone(),
                base_url: env.base_url.clone(),
                model: env.model.clone(),
                description: env.description.clone(),
            };

            self.environments.insert(env.name.clone(), llm_env);
        }

        Ok(())
    }
}

impl EnvironmentManager for LlmEnvironmentManager {
    fn environment_type(&self) -> EnvironmentType {
        EnvironmentType::Llm
    }

    fn list(&self) -> Result<Vec<DynEnvironment>, String> {
        let mut result = Vec::new();
        for env in self.environments.values() {
            result.push(DynEnvironment {
                name: env.name.clone(),
                path: env.base_url.clone(),
                version: Some(env.model.clone()),
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
                version: Some(env.model.clone()),
                description: Some(env.description.clone()),
                is_active: env.is_active(),
            }))
        } else {
            Ok(None)
        }
    }

    fn add(&mut self, name: &str, config_str: &str) -> Result<(), String> {
        // Parse config as JSON
        let config: serde_json::Value = serde_json::from_str(config_str)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        let provider = config
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("openai");

        let api_key = config
            .get("api_key")
            .and_then(|v| v.as_str())
            .ok_or("Missing api_key in config")?;

        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1");

        let model = config
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("gpt-3.5-turbo");

        let temperature = config.get("temperature").and_then(|v| v.as_f64());
        let max_tokens = config
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let default_desc = format!("LLM: {} ({})", name, model);
        let description = config
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_desc);

        // Create LLM environment
        let llm_environment = LlmEnvironment {
            name: name.to_string(),
            provider: provider.to_string(),
            description: description.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        };

        // 持久化到配置文件
        let mut file_config = Config::load().map_err(|e| format!("Failed to load config: {}", e))?;
        if let Some(existing) = file_config
            .llm_environments
            .iter_mut()
            .find(|env| env.name == name)
        {
            existing.provider = provider.to_string();
            existing.api_key = api_key.to_string();
            existing.base_url = base_url.to_string();
            existing.model = model.to_string();
            existing.description = description.to_string();
            existing.temperature = temperature;
            existing.max_tokens = max_tokens;
        } else {
            file_config.llm_environments.push(ConfigLlmEnvironment {
                name: name.to_string(),
                provider: provider.to_string(),
                api_key: api_key.to_string(),
                base_url: base_url.to_string(),
                model: model.to_string(),
                temperature,
                max_tokens,
                description: description.to_string(),
            });
        }

        file_config
            .save()
            .map_err(|e| format!("Failed to save config: {}", e))?;

        self.environments.insert(name.to_string(), llm_environment);
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        if self.environments.remove(name).is_none() {
            return Err(format!("LLM environment '{}' not found", name));
        }

        let mut config = Config::load().map_err(|e| format!("Failed to load config: {}", e))?;
        let original_len = config.llm_environments.len();
        config.llm_environments.retain(|env| env.name != name);
        if config.llm_environments.len() == original_len {
            return Err(format!("LLM environment '{}' not found", name));
        }

        config
            .save()
            .map_err(|e| format!("Failed to save config: {}", e))?;

        Ok(())
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, String> {
        let llm_env = self
            .environments
            .get(name)
            .ok_or_else(|| format!("LLM environment '{}' not found", name))?;

        let shell_type =
            shell_type.unwrap_or_else(crate::infrastructure::shell::platform::detect_shell);

        // Create config for script generation
        let mut config = serde_json::json!({
            "api_key": llm_env.api_key,
            "base_url": llm_env.base_url,
            "model": llm_env.model,
        });

        // Add Anthropic-specific environment variables for GLM_CC
        if llm_env.provider == "anthropic" {
            // For Anthropic/GLM_CC, set specific environment variables
            let auth_token = if llm_env.api_key.starts_with("${") {
                llm_env.resolve_env_var(&llm_env.api_key)
            } else {
                llm_env.api_key.clone()
            };

            let base_url = if llm_env.base_url.starts_with("${") {
                llm_env.resolve_env_var(&llm_env.base_url)
            } else {
                llm_env.base_url.clone()
            };

            config["anthropic_auth_token"] = serde_json::Value::String(auth_token);
            config["anthropic_base_url"] = serde_json::Value::String(base_url);
            config["api_timeout_ms"] = serde_json::Value::String("3000000".to_string());
            config["claude_code_disable_nonessential_traffic"] =
                serde_json::Value::Number(serde_json::Number::from(1));
        }

        let generator = ScriptGenerator::new().map_err(|e| e.to_string())?;
        match generator.generate_switch_script(EnvironmentType::Llm, name, &config, Some(shell_type)) {
            Ok(script) => Ok(script),
            Err(e) => Err(format!("Failed to generate script: {}", e)),
        }
    }

    fn get_current(&self) -> Result<Option<String>, String> {
        // Session 为主
        if let Ok(session) = SessionManager::new() {
            if let Some(current) = session.get_current_environment(EnvironmentType::Llm) {
                return Ok(Some(current.clone()));
            }
        }

        // 兜底：根据环境变量推测
        if let Ok(_api_key) = std::env::var("OPENAI_API_KEY") {
            for (name, llm_env) in &self.environments {
                if let Ok(current_api_key) = std::env::var("OPENAI_API_KEY") {
                    if current_api_key == llm_env.resolve_env_var(&llm_env.api_key) {
                        return Ok(Some(name.clone()));
                    }
                }
            }
        }

        Ok(None)
    }

    fn scan(&self) -> Result<Vec<DynEnvironment>, String> {
        let mut result = Vec::new();

        // "Scan" for LLM environments by checking Anthropic environment variables
        if let (Ok(auth_token), Ok(base_url)) = (
            std::env::var("ANTHROPIC_AUTH_TOKEN"),
            std::env::var("ANTHROPIC_BASE_URL"),
        ) {
            let llm_env = LlmEnvironment {
                name: "anthropic-detected".to_string(),
                provider: "anthropic".to_string(),
                description: "Detected Anthropic environment from system variables".to_string(),
                api_key: auth_token,
                base_url: base_url,
                model: std::env::var("ANTHROPIC_MODEL")
                    .unwrap_or_else(|_| "claude-3-sonnet-20240229".to_string()),
            };
            result.push(DynEnvironment {
                name: llm_env.name.clone(),
                path: llm_env.base_url.clone(),
                version: Some(llm_env.model.clone()),
                description: Some(llm_env.description.clone()),
                is_active: llm_env.is_active(),
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

/// LLM Environment representation
#[derive(Debug, Clone)]
struct LlmEnvironment {
    name: String,
    provider: String,
    description: String,
    api_key: String,
    base_url: String,
    model: String,
}

impl LlmEnvironment {
    fn is_active(&self) -> bool {
        // Check if this environment is currently active (focus on Anthropic)
        match self.provider.as_str() {
            "anthropic" => {
                // For Anthropic, check ANTHROPIC_AUTH_TOKEN and ANTHROPIC_BASE_URL
                if let (Ok(current_token), Ok(current_base_url)) = (
                    std::env::var("ANTHROPIC_AUTH_TOKEN"),
                    std::env::var("ANTHROPIC_BASE_URL"),
                ) {
                    // Compare both token and base URL
                    let env_token = self.resolve_env_var(&self.api_key);
                    let env_base_url = self.resolve_env_var(&self.base_url);

                    current_token == env_token && current_base_url == env_base_url
                } else {
                    false
                }
            }
            _ => false, // Currently only support Anthropic detection
        }
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
