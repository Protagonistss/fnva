//! Claude Code 只说 Anthropic Messages API 一种协议,所以每个 CC 环境的配置
//! 构建方式都相同:把 `ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_BASE_URL` 及模型
//! 覆写写进切换脚本用的 JSON config。不再需要 provider 抽象。

use crate::infrastructure::config::CcEnvironment as ConfigCcEnvironment;
use serde_json;

/// 构建切换脚本模板消费的 JSON config。
///
/// `api_key` / `base_url` 可以是字面量,也可以是 `${ENV_VAR}` 引用(切换时解析)。
/// 模型字段可选。
pub fn apply_anthropic_config(env: &ConfigCcEnvironment, config: &mut serde_json::Value) {
    let auth_token_raw = &env.api_key;
    let base_url_resolved = if env.base_url.starts_with("${") {
        env.resolve_env_var(&env.base_url)
    } else {
        env.base_url.clone()
    };

    config["anthropic_auth_token"] = serde_json::Value::String(auth_token_raw.clone());
    config["anthropic_base_url"] = serde_json::Value::String(base_url_resolved);

    let timeout_ms = env
        .api_timeout_ms
        .as_deref()
        .unwrap_or("3000000")
        .to_string();
    config["api_timeout_ms"] = serde_json::Value::String(timeout_ms);
    config["claude_code_disable_nonessential_traffic"] =
        serde_json::Value::Number(serde_json::Number::from(1));

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

/// 当前 shell 是否已经激活了某个 Anthropic 协议的 CC 环境。
pub fn is_anthropic_active() -> bool {
    std::env::var("ANTHROPIC_AUTH_TOKEN").is_ok()
}
