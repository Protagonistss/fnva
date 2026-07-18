use serde::{Deserialize, Serialize};

/// 默认 CC sonnet 模型名(配置缺省值与扫描兜底共用)。
pub const DEFAULT_SONNET_MODEL: &str = "claude-sonnet-4-5";

/// 环境来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum EnvironmentSource {
    #[serde(rename = "manual")]
    #[default]
    Manual,
    #[serde(rename = "scanned")]
    Scanned,
}

/// Java 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaEnvironment {
    pub name: String,
    pub java_home: String,
    #[serde(default)]
    pub description: String,
    /// 环境来源：manual（手动添加）或 scanned（扫描发现）
    #[serde(default)]
    pub source: EnvironmentSource,
}

/// Maven 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavenEnvironment {
    pub name: String,
    pub maven_home: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source: EnvironmentSource,
    /// 自定义 MAVEN_OPTS（JVM 参数，如 -Xmx4g -Dfile.encoding=UTF-8）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maven_opts: Option<String>,
    /// 自定义本地仓库路径（替代默认 ~/.m2/repository）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_repo: Option<String>,
    /// 自定义 settings.xml 路径
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings_file: Option<String>,
}

/// CC (Claude Code) 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcEnvironment {
    pub name: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub sonnet_model: String,
    #[serde(default)]
    pub opus_model: Option<String>,
    #[serde(default)]
    pub haiku_model: Option<String>,
    #[serde(default)]
    pub description: String,
    /// API timeout in milliseconds (default: 3000000 = 50 min, for long Claude Code requests)
    #[serde(default)]
    pub api_timeout_ms: Option<String>,
    /// Extra environment variables to export verbatim (e.g. CLAUDE_CODE_AUTO_COMPACT_WINDOW)
    #[serde(default)]
    pub extra_env: std::collections::HashMap<String, String>,
}

/// 默认 CC 环境配置（仅保留一个 anthropic-cc 作为初始示例）
pub fn default_cc_environments() -> Vec<CcEnvironment> {
    vec![CcEnvironment {
        name: "anthropic-cc".to_string(),
        api_key: "${ANTHROPIC_API_KEY}".to_string(),
        base_url: "https://api.anthropic.com".to_string(),
        sonnet_model: DEFAULT_SONNET_MODEL.to_string(),
        opus_model: Some("claude-opus-4-5".to_string()),
        haiku_model: Some("claude-haiku-4-5".to_string()),
        description: "Anthropic Claude Code environment".to_string(),
        api_timeout_ms: None,
        extra_env: std::collections::HashMap::new(),
    }]
}
