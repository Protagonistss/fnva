use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// LLM 提供商接口（对象安全）
pub trait LlmProvider: Send + Sync {
    /// 获取提供商名称
    fn name(&self) -> &str;

    /// 验证 API 密钥格式
    fn validate_api_key(&self, api_key: &str) -> Result<(), String>;

    /// 获取默认模型列表
    fn default_models(&self) -> Vec<String>;

    /// 获取 API 端点
    fn api_endpoint(&self) -> Option<&str>;

    /// 生成环境变量
    fn generate_env_vars(&self, config: &LlmProviderConfig) -> HashMap<String, String>;
}

/// LLM 提供商异步接口
pub trait LlmProviderAsync: LlmProvider {
    /// 测试连接
    async fn test_connection(&self, config: &LlmProviderConfig) -> Result<(), String>;
}

/// LLM 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub timeout: Option<u64>,
}

/// OpenAI 提供商
pub struct OpenAIProvider;

impl LlmProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn validate_api_key(&self, api_key: &str) -> Result<(), String> {
        if !api_key.starts_with("sk-") {
            return Err("OpenAI API key should start with 'sk-'".to_string());
        }
        if api_key.len() < 20 {
            return Err("OpenAI API key seems too short".to_string());
        }
        Ok(())
    }

    fn default_models(&self) -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-4-32k".to_string(),
            "gpt-3.5-turbo".to_string(),
            "gpt-3.5-turbo-16k".to_string(),
            "text-davinci-003".to_string(),
        ]
    }

    fn api_endpoint(&self) -> Option<&str> {
        Some("https://api.openai.com/v1")
    }

    fn generate_env_vars(&self, config: &LlmProviderConfig) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();
        env_vars.insert("OPENAI_API_KEY".to_string(), config.api_key.clone());

        if let Some(model) = &config.model {
            env_vars.insert("OPENAI_MODEL".to_string(), model.clone());
        }

        if let Some(base_url) = &config.base_url {
            env_vars.insert("OPENAI_BASE_URL".to_string(), base_url.clone());
        }

        env_vars
    }
}

impl LlmProviderAsync for OpenAIProvider {
    async fn test_connection(&self, _config: &LlmProviderConfig) -> Result<(), String> {
        // TODO: 实现实际的连接测试
        Ok(())
    }
}

/// Anthropic 提供商
pub struct AnthropicProvider;

impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn validate_api_key(&self, api_key: &str) -> Result<(), String> {
        if !api_key.starts_with("sk-ant-") {
            return Err("Anthropic API key should start with 'sk-ant-'".to_string());
        }
        Ok(())
    }

    fn default_models(&self) -> Vec<String> {
        vec![
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
            "claude-2.1".to_string(),
            "claude-2.0".to_string(),
        ]
    }

    fn api_endpoint(&self) -> Option<&str> {
        Some("https://api.anthropic.com/v1")
    }

    fn generate_env_vars(&self, config: &LlmProviderConfig) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();
        env_vars.insert("ANTHROPIC_API_KEY".to_string(), config.api_key.clone());

        if let Some(model) = &config.model {
            env_vars.insert("ANTHROPIC_MODEL".to_string(), model.clone());
        }

        env_vars
    }
}

impl LlmProviderAsync for AnthropicProvider {
    async fn test_connection(&self, _config: &LlmProviderConfig) -> Result<(), String> {
        // TODO: 实现实际的连接测试
        Ok(())
    }
}

/// Azure OpenAI 提供商
pub struct AzureOpenAIProvider;

impl LlmProvider for AzureOpenAIProvider {
    fn name(&self) -> &str {
        "azure-openai"
    }

    fn validate_api_key(&self, api_key: &str) -> Result<(), String> {
        // Azure OpenAI API keys are typically 32 character hex strings
        if api_key.len() != 32 {
            return Err("Azure OpenAI API key should be 32 characters long".to_string());
        }
        Ok(())
    }

    fn default_models(&self) -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-4-32k".to_string(),
            "gpt-35-turbo".to_string(),
            "text-davinci-003".to_string(),
        ]
    }

    fn api_endpoint(&self) -> Option<&str> {
        None // Azure OpenAI requires custom endpoint
    }

    fn generate_env_vars(&self, config: &LlmProviderConfig) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();
        env_vars.insert("AZURE_OPENAI_API_KEY".to_string(), config.api_key.clone());

        if let Some(base_url) = &config.base_url {
            env_vars.insert("AZURE_OPENAI_ENDPOINT".to_string(), base_url.clone());
        }

        if let Some(model) = &config.model {
            env_vars.insert("AZURE_OPENAI_DEPLOYMENT_NAME".to_string(), model.clone());
        }

        env_vars
    }
}

impl LlmProviderAsync for AzureOpenAIProvider {
    async fn test_connection(&self, _config: &LlmProviderConfig) -> Result<(), String> {
        // TODO: 实现实际的连接测试
        Ok(())
    }
}

/// Google Gemini 提供商
pub struct GoogleGeminiProvider;

impl LlmProvider for GoogleGeminiProvider {
    fn name(&self) -> &str {
        "google-gemini"
    }

    fn validate_api_key(&self, api_key: &str) -> Result<(), String> {
        // Google API keys are typically alphanumeric
        if !api_key.chars().all(|c| c.is_alphanumeric()) {
            return Err("Google API key should be alphanumeric".to_string());
        }
        Ok(())
    }

    fn default_models(&self) -> Vec<String> {
        vec![
            "gemini-pro".to_string(),
            "gemini-pro-vision".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ]
    }

    fn api_endpoint(&self) -> Option<&str> {
        Some("https://generativelanguage.googleapis.com/v1")
    }

    fn generate_env_vars(&self, config: &LlmProviderConfig) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();
        env_vars.insert("GOOGLE_API_KEY".to_string(), config.api_key.clone());

        if let Some(model) = &config.model {
            env_vars.insert("GOOGLE_MODEL".to_string(), model.clone());
        }

        env_vars
    }
}

impl LlmProviderAsync for GoogleGeminiProvider {
    async fn test_connection(&self, _config: &LlmProviderConfig) -> Result<(), String> {
        // TODO: 实现实际的连接测试
        Ok(())
    }
}

/// 提供商工厂
pub struct ProviderFactory;

impl ProviderFactory {
    /// 创建提供商实例
    pub fn create_provider(provider_name: &str) -> Result<Box<dyn LlmProvider>, String> {
        match provider_name.to_lowercase().as_str() {
            "openai" => Ok(Box::new(OpenAIProvider)),
            "anthropic" => Ok(Box::new(AnthropicProvider)),
            "azure-openai" => Ok(Box::new(AzureOpenAIProvider)),
            "google-gemini" => Ok(Box::new(GoogleGeminiProvider)),
            _ => Err(format!("Unsupported provider: {}", provider_name)),
        }
    }

    /// 获取所有支持的提供商
    pub fn get_supported_providers() -> Vec<&'static str> {
        vec![
            "openai",
            "anthropic",
            "azure-openai",
            "google-gemini",
        ]
    }

    /// 验证提供商名称
    pub fn validate_provider(provider_name: &str) -> Result<(), String> {
        if Self::get_supported_providers()
            .iter()
            .any(|&p| p == provider_name.to_lowercase().as_str()) {
            Ok(())
        } else {
            Err(format!(
                "Unsupported provider: {}. Supported: {}",
                provider_name,
                Self::get_supported_providers().join(", ")
            ))
        }
    }
}