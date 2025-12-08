use crate::utils::validation::ValidationUtils;
use url::Url;

/// LLM 环境验证器
pub struct LlmValidator;

impl LlmValidator {
    /// 验证 LLM 环境配置
    pub fn validate_environment(
        name: &str,
        provider: &str,
        api_key: &str,
        base_url: Option<&str>,
        model: Option<&str>,
    ) -> Result<(), String> {
        // 验证环境名称
        ValidationUtils::validate_environment_name(name)?;

        // 验证提供商
        Self::validate_provider(provider)?;

        // 验证 API Key
        ValidationUtils::validate_api_key(api_key)?;

        // 验证 Base URL（如果提供）
        if let Some(url) = base_url {
            ValidationUtils::validate_url(url)?;
            Self::validate_provider_base_url(provider, url)?;
        }

        // 验证模型名称（如果提供）
        if let Some(model_name) = model {
            Self::validate_model_name(provider, model_name)?;
        }

        Ok(())
    }

    /// 验证 LLM 提供商
    pub fn validate_provider(provider: &str) -> Result<(), String> {
        let valid_providers = [
            "openai",
            "anthropic",
            "azure-openai",
            "google-gemini",
            "cohere",
            "mistral",
            "ollama",
            "huggingface",
            "baidu",
            "alibaba",
            "tencent",
        ];

        for valid_provider in &valid_providers {
            if provider.to_lowercase() == *valid_provider {
                return Ok(());
            }
        }

        Err(format!(
            "Unsupported provider: '{}'. Supported providers: {}",
            provider,
            valid_providers.join(", ")
        ))
    }

    /// 验证提供商的 Base URL
    pub fn validate_provider_base_url(provider: &str, base_url: &str) -> Result<(), String> {
        let url = Url::parse(base_url).map_err(|e| format!("Invalid URL: {e}"))?;

        match provider.to_lowercase().as_str() {
            "openai" => {
                if !url.host_str().unwrap_or("").contains("openai") {
                    eprintln!("Warning: OpenAI provider typically uses api.openai.com");
                }
            }
            "anthropic" => {
                if !url.host_str().unwrap_or("").contains("anthropic") {
                    eprintln!("Warning: Anthropic provider typically uses api.anthropic.com");
                }
            }
            "azure-openai" => {
                if !url.host_str().unwrap_or("").contains("azure") {
                    eprintln!("Warning: Azure OpenAI provider should use Azure endpoints");
                }
            }
            "google-gemini" => {
                if !url.host_str().unwrap_or("").contains("google") {
                    eprintln!("Warning: Google Gemini provider typically uses googleapis.com");
                }
            }
            _ => {
                // 其他提供商的 URL 检查
            }
        }

        Ok(())
    }

    /// 验证模型名称
    pub fn validate_model_name(provider: &str, model: &str) -> Result<(), String> {
        if model.is_empty() {
            return Err("Model name cannot be empty".to_string());
        }

        if model.len() > 100 {
            return Err("Model name too long".to_string());
        }

        // 验证特定提供商的模型名称格式
        match provider.to_lowercase().as_str() {
            "openai" => {
                Self::validate_openai_model(model)?;
            }
            "anthropic" => {
                Self::validate_anthropic_model(model)?;
            }
            "azure-openai" => {
                Self::validate_azure_model(model)?;
            }
            _ => {
                // 其他提供商的通用验证
            }
        }

        Ok(())
    }

    /// 验证 OpenAI 模型名称
    fn validate_openai_model(model: &str) -> Result<(), String> {
        let valid_prefixes = [
            "gpt-", "text-", "code-", "davinci-", "curie-", "babbage-", "ada-",
        ];
        let valid_models = [
            "gpt-4",
            "gpt-4-32k",
            "gpt-3.5-turbo",
            "gpt-3.5-turbo-16k",
            "text-davinci-003",
            "text-curie-001",
            "text-babbage-001",
            "text-ada-001",
        ];

        let model_lower = model.to_lowercase();

        // 检查是否匹配已知的模型或前缀
        if valid_models.iter().any(|m| *m == model_lower)
            || valid_prefixes
                .iter()
                .any(|prefix| model_lower.starts_with(prefix))
        {
            return Ok(());
        }

        eprintln!("Warning: Unusual OpenAI model name: {model}");
        Ok(())
    }

    /// 验证 Anthropic 模型名称
    fn validate_anthropic_model(model: &str) -> Result<(), String> {
        let valid_prefixes = ["claude-", "anthropic-"];
        let model_lower = model.to_lowercase();

        if valid_prefixes
            .iter()
            .any(|prefix| model_lower.starts_with(prefix))
        {
            return Ok(());
        }

        eprintln!("Warning: Unusual Anthropic model name: {model}");
        Ok(())
    }

    /// 验证 Azure OpenAI 模型名称
    fn validate_azure_model(model: &str) -> Result<(), String> {
        // Azure OpenAI 模型名称通常是部署名称
        if model.is_empty() {
            return Err("Azure OpenAI deployment name cannot be empty".to_string());
        }

        // 检查是否包含非法字符
        let invalid_chars = ['/', '\\', '?', '#', '%', '"'];
        for &ch in &invalid_chars {
            if model.contains(ch) {
                return Err(format!("Azure deployment name cannot contain '{ch}'"));
            }
        }

        if model.len() > 50 {
            return Err("Azure deployment name too long (max 50 characters)".to_string());
        }

        Ok(())
    }

    /// 验证温度参数
    pub fn validate_temperature(temperature: f64) -> Result<(), String> {
        ValidationUtils::validate_temperature(temperature)
    }

    /// 验证 max_tokens 参数
    pub fn validate_max_tokens(max_tokens: u32) -> Result<(), String> {
        ValidationUtils::validate_max_tokens(max_tokens)
    }

    /// 验证 API 密钥格式
    pub fn validate_api_key_format(provider: &str, api_key: &str) -> Result<(), String> {
        // 检查是否是环境变量引用
        if api_key.starts_with("${") && api_key.ends_with('}') {
            return Ok(());
        }

        match provider.to_lowercase().as_str() {
            "openai" => {
                if !api_key.starts_with("sk-") {
                    eprintln!("Warning: OpenAI API keys typically start with 'sk-'");
                }
            }
            "anthropic" => {
                if !api_key.starts_with("sk-ant-") {
                    eprintln!("Warning: Anthropic API keys typically start with 'sk-ant-'");
                }
            }
            "azure-openai" => {
                // Azure OpenAI 通常使用 Key，格式可能不同
            }
            _ => {
                // 其他提供商的 API 密钥格式检查
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_provider() {
        assert!(LlmValidator::validate_provider("openai").is_ok());
        assert!(LlmValidator::validate_provider("anthropic").is_ok());
        assert!(LlmValidator::validate_provider("invalid").is_err());
    }

    #[test]
    fn test_validate_model_name() {
        assert!(LlmValidator::validate_model_name("openai", "gpt-4").is_ok());
        assert!(LlmValidator::validate_model_name("anthropic", "claude-3").is_ok());
        assert!(LlmValidator::validate_model_name("openai", "").is_err());
    }

    #[test]
    fn test_validate_temperature() {
        assert!(LlmValidator::validate_temperature(0.7).is_ok());
        assert!(LlmValidator::validate_temperature(1.5).is_ok());
        assert!(LlmValidator::validate_temperature(-0.1).is_err());
        assert!(LlmValidator::validate_temperature(2.1).is_err());
    }

    #[test]
    fn test_validate_max_tokens() {
        assert!(LlmValidator::validate_max_tokens(1000).is_ok());
        assert!(LlmValidator::validate_max_tokens(0).is_err());
        assert!(LlmValidator::validate_max_tokens(50000).is_err());
    }
}
