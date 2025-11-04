use crate::config::{Config, LlmEnvironment, resolve_env_var};
use crate::platform::{generate_env_command, detect_shell, ShellType};
use std::collections::HashMap;

/// LLM 环境管理器
pub struct LlmManager;

impl LlmManager {
    /// 列出所有 LLM 环境
    pub fn list(config: &Config) -> Vec<&LlmEnvironment> {
        config.llm_environments.iter().collect()
    }

    /// 生成切换到指定 LLM 环境的命令
    pub fn generate_switch_command(
        config: &Config,
        name: &str,
        shell: Option<ShellType>,
    ) -> Result<String, String> {
        let env = config
            .get_llm_env(name)
            .ok_or_else(|| format!("LLM 环境 '{}' 不存在", name))?;

        let shell = shell.unwrap_or_else(detect_shell);
        let mut commands = Vec::new();

        // 根据提供商生成相应的环境变量
        let env_vars = generate_env_vars_for_provider(env);

        for (key, value) in env_vars {
            commands.push(generate_env_command(&key, &value, shell));
        }

        Ok(commands.join("\n"))
    }

    /// 添加 LLM 环境到配置
    pub fn add(
        config: &mut Config,
        name: String,
        provider: String,
        api_key: Option<String>,
        base_url: Option<String>,
        model: Option<String>,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        description: Option<String>,
    ) -> Result<(), String> {
        let env = LlmEnvironment {
            name,
            provider: provider.clone(),
            api_key: api_key.unwrap_or_default(),
            base_url: base_url.unwrap_or_else(|| get_default_base_url(&provider)),
            model: model.unwrap_or_default(),
            temperature,
            max_tokens,
            description: description.unwrap_or_default(),
        };

        config.add_llm_env(env)?;
        config.save()?;
        Ok(())
    }

    /// 从配置中删除 LLM 环境
    pub fn remove(config: &mut Config, name: &str) -> Result<(), String> {
        config.remove_llm_env(name)?;
        config.save()?;
        Ok(())
    }

    /// 获取支持的提供商列表
    pub fn get_providers() -> Vec<&'static str> {
        vec![
            "openai",
            "anthropic",
            "azure-openai",
            "google-gemini",
            "cohere",
            "mistral",
            "ollama",
        ]
    }
}

/// 根据提供商生成环境变量映射
fn generate_env_vars_for_provider(env: &LlmEnvironment) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // 解析 API Key（支持环境变量引用）
    let api_key = resolve_env_var(&env.api_key);

    match env.provider.as_str() {
        "openai" => {
            vars.insert("OPENAI_API_KEY".to_string(), api_key);
            if !env.base_url.is_empty() && env.base_url != "https://api.openai.com/v1" {
                vars.insert("OPENAI_BASE_URL".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("OPENAI_MODEL".to_string(), env.model.clone());
            }
        }
        "anthropic" => {
            vars.insert("ANTHROPIC_API_KEY".to_string(), api_key);
            if !env.base_url.is_empty() && env.base_url != "https://api.anthropic.com" {
                vars.insert("ANTHROPIC_BASE_URL".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("ANTHROPIC_MODEL".to_string(), env.model.clone());
            }
        }
        "azure-openai" => {
            vars.insert("AZURE_OPENAI_API_KEY".to_string(), api_key);
            if !env.base_url.is_empty() {
                vars.insert("AZURE_OPENAI_ENDPOINT".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("AZURE_OPENAI_DEPLOYMENT_NAME".to_string(), env.model.clone());
            }
        }
        "google-gemini" => {
            vars.insert("GOOGLE_API_KEY".to_string(), api_key);
            if !env.base_url.is_empty() {
                vars.insert("GOOGLE_GEMINI_BASE_URL".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("GOOGLE_GEMINI_MODEL".to_string(), env.model.clone());
            }
        }
        "cohere" => {
            vars.insert("COHERE_API_KEY".to_string(), api_key);
            if !env.base_url.is_empty() {
                vars.insert("COHERE_BASE_URL".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("COHERE_MODEL".to_string(), env.model.clone());
            }
        }
        "mistral" => {
            vars.insert("MISTRAL_API_KEY".to_string(), api_key);
            if !env.base_url.is_empty() {
                vars.insert("MISTRAL_BASE_URL".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("MISTRAL_MODEL".to_string(), env.model.clone());
            }
        }
        "ollama" => {
            if !env.base_url.is_empty() {
                vars.insert("OLLAMA_BASE_URL".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("OLLAMA_MODEL".to_string(), env.model.clone());
            }
        }
        _ => {
            // 通用环境变量
            vars.insert("LLM_API_KEY".to_string(), api_key);
            if !env.base_url.is_empty() {
                vars.insert("LLM_BASE_URL".to_string(), env.base_url.clone());
            }
            if !env.model.is_empty() {
                vars.insert("LLM_MODEL".to_string(), env.model.clone());
            }
        }
    }

    // 添加通用参数
    if let Some(temp) = env.temperature {
        vars.insert("LLM_TEMPERATURE".to_string(), temp.to_string());
    }
    if let Some(max) = env.max_tokens {
        vars.insert("LLM_MAX_TOKENS".to_string(), max.to_string());
    }

    // 添加提供商信息
    vars.insert("LLM_PROVIDER".to_string(), env.provider.clone());

    vars
}

/// 获取提供商的默认 Base URL
fn get_default_base_url(provider: &str) -> String {
    match provider {
        "openai" => "https://api.openai.com/v1".to_string(),
        "anthropic" => "https://api.anthropic.com".to_string(),
        "azure-openai" => String::new(), // Azure 需要用户指定
        "google-gemini" => "https://generativelanguage.googleapis.com/v1".to_string(),
        "cohere" => "https://api.cohere.ai/v1".to_string(),
        "mistral" => "https://api.mistral.ai/v1".to_string(),
        "ollama" => "http://localhost:11434".to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_providers() {
        let providers = LlmManager::get_providers();
        assert!(providers.contains(&"openai"));
        assert!(providers.contains(&"anthropic"));
    }

    #[test]
    fn test_get_default_base_url() {
        assert_eq!(
            get_default_base_url("openai"),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            get_default_base_url("anthropic"),
            "https://api.anthropic.com"
        );
    }
}
