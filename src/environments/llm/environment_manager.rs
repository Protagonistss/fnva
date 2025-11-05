use std::collections::HashMap;
use crate::core::environment_manager::{EnvironmentManager, EnvironmentType, DynEnvironment};
use crate::infrastructure::shell::ShellType;
use crate::infrastructure::shell::script_builder::ScriptBuilder;
use serde_json;

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

        // Initialize with some default LLM configurations if they exist
        // This is a simplified implementation - in practice you'd load from config files
        manager
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

        let api_key = config.get("api_key")
            .and_then(|v| v.as_str())
            .ok_or("Missing api_key in config")?;

        let base_url = config.get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1");

        let model = config.get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("gpt-3.5-turbo");

        let default_desc = format!("LLM: {} ({})", name, model);
        let description = config.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_desc);

        // Create LLM environment
        let llm_environment = LlmEnvironment {
            name: name.to_string(),
            description: description.to_string(),
            api_key: api_key.to_string(),
            base_url: base_url.to_string(),
            model: model.to_string(),
        };

        self.environments.insert(name.to_string(), llm_environment);
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        if self.environments.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("LLM environment '{}' not found", name))
        }
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, String> {
        let llm_env = self.environments.get(name)
            .ok_or_else(|| format!("LLM environment '{}' not found", name))?;

        let shell_type = shell_type.unwrap_or_else(crate::infrastructure::shell::platform::detect_shell);

        // Create config for script generation
        let config = serde_json::json!({
            "api_key": llm_env.api_key,
            "base_url": llm_env.base_url,
            "model": llm_env.model,
        });

        ScriptBuilder::build_switch_script(
            EnvironmentType::Llm,
            name,
            &config,
            shell_type
        )
    }

    fn get_current(&self) -> Result<Option<String>, String> {
        // Check environment variables to determine current LLM
        if let Ok(_api_key) = std::env::var("OPENAI_API_KEY") {
            // Try to identify which environment based on other variables
            for (name, llm_env) in &self.environments {
                if let Ok(current_api_key) = std::env::var("OPENAI_API_KEY") {
                    if current_api_key == llm_env.api_key {
                        return Ok(Some(name.clone()));
                    }
                }
            }
        }
        Ok(None)
    }

    fn scan(&self) -> Result<Vec<DynEnvironment>, String> {
        let mut result = Vec::new();

        // "Scan" for LLM environments by checking environment variables
        // This is a simplified implementation
        if std::env::var("OPENAI_API_KEY").is_ok() {
            let llm_env = LlmEnvironment {
                name: "default".to_string(),
                description: "Default OpenAI environment".to_string(),
                api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
                base_url: std::env::var("OPENAI_BASE_URL").unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
                model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
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

    fn set_current(&mut self, name: &str) -> Result<(), String> {
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
    description: String,
    api_key: String,
    base_url: String,
    model: String,
}

impl LlmEnvironment {
    fn is_active(&self) -> bool {
        // Check if this environment is currently active
        if let Ok(current_api_key) = std::env::var("OPENAI_API_KEY") {
            current_api_key == self.api_key
        } else {
            false
        }
    }
}