use crate::infrastructure::config::CcEnvironment as ConfigCcEnvironment;

pub trait CcProvider {
    /// Append provider-specific variables to the config JSON object
    fn setup_config(&self, env: &ConfigCcEnvironment, config: &mut serde_json::Value);
    
    /// Check if the environment looks like it's active for this provider based on environment variables
    fn is_active(&self) -> bool;
}

pub struct AnthropicProvider;

impl CcProvider for AnthropicProvider {
    fn setup_config(&self, env: &ConfigCcEnvironment, config: &mut serde_json::Value) {
        let auth_token_raw = &env.api_key;
        let base_url_resolved = if env.base_url.starts_with("${") {
            env.resolve_env_var(&env.base_url)
        } else {
            env.base_url.clone()
        };

        config["anthropic_auth_token"] = serde_json::Value::String(auth_token_raw.clone());
        config["anthropic_base_url"] = serde_json::Value::String(base_url_resolved);

        let timeout_ms = env.api_timeout_ms.as_deref().unwrap_or("3000000").to_string();
        config["api_timeout_ms"] = serde_json::Value::String(timeout_ms);
        config["claude_code_disable_nonessential_traffic"] = serde_json::Value::Number(serde_json::Number::from(1));

        if !env.sonnet_model.is_empty() {
            let opus_model = env.opus_model.as_ref().unwrap_or(&env.sonnet_model);
            let sonnet_model = &env.sonnet_model;
            let haiku_model = env.haiku_model.as_ref().unwrap_or(&env.sonnet_model);

            config["opus_model"] = serde_json::Value::String(opus_model.clone());
            config["sonnet_model"] = serde_json::Value::String(sonnet_model.clone());
            config["haiku_model"] = serde_json::Value::String(haiku_model.clone());
            config["default_model"] = serde_json::Value::String(sonnet_model.clone());
        }

        for (k, v) in &env.extra_env {
            config[k.to_lowercase()] = serde_json::Value::String(v.clone());
        }
    }

    fn is_active(&self) -> bool {
        std::env::var("ANTHROPIC_AUTH_TOKEN").is_ok()
    }
}

pub struct GenericProvider;

impl CcProvider for GenericProvider {
    fn setup_config(&self, env: &ConfigCcEnvironment, config: &mut serde_json::Value) {
        for (k, v) in &env.extra_env {
            config[k.to_lowercase()] = serde_json::Value::String(v.clone());
        }
    }

    fn is_active(&self) -> bool {
        false
    }
}

pub fn get_provider(provider_name: &str) -> Box<dyn CcProvider> {
    match provider_name.to_lowercase().as_str() {
        "anthropic" => Box::new(AnthropicProvider),
        _ => Box::new(GenericProvider),
    }
}
