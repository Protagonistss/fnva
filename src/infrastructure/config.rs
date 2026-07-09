use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// 默认 CC sonnet 模型名(配置缺省值与扫描兜底共用)。
pub const DEFAULT_SONNET_MODEL: &str = "claude-sonnet-4-5";

/// 镜像配置（模板化 URL）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub name: String,
    #[serde(default = "default_mirror_priority")]
    pub priority: u32,
    pub base_url: String,
    /// URL 模板变量: {base_url}, {major}, {tag}, {filename}, {os}, {arch}
    pub url_template: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_mirror_priority() -> u32 {
    10
}

fn default_true() -> bool {
    true
}

/// 所有工具的镜像配置集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorsConfig {
    #[serde(default = "default_java_mirrors")]
    pub java: Vec<MirrorConfig>,
    #[serde(default = "default_maven_mirrors")]
    pub maven: Vec<MirrorConfig>,
}

impl Default for MirrorsConfig {
    fn default() -> Self {
        Self {
            java: default_java_mirrors(),
            maven: default_maven_mirrors(),
        }
    }
}

impl MirrorsConfig {
    /// 通用访问器:按工具 id 取镜像列表(未知工具返回空切片)。
    pub fn get(&self, tool: &str) -> &[MirrorConfig] {
        match tool {
            "java" => &self.java,
            "maven" => &self.maven,
            _ => &[],
        }
    }
}

fn default_maven_mirrors() -> Vec<MirrorConfig> {
    vec![
        MirrorConfig {
            name: "tsinghua".to_string(),
            priority: 1,
            base_url: "https://mirrors.tuna.tsinghua.edu.cn/apache/maven/maven-3".to_string(),
            url_template: "{base_url}/{version}/binaries/apache-maven-{version}-bin.tar.gz"
                .to_string(),
            enabled: true,
        },
        MirrorConfig {
            name: "apache-archive".to_string(),
            priority: 2,
            base_url: "https://archive.apache.org/dist/maven/maven-3".to_string(),
            url_template: "{base_url}/{version}/binaries/apache-maven-{version}-bin.tar.gz"
                .to_string(),
            enabled: true,
        },
    ]
}

fn default_java_mirrors() -> Vec<MirrorConfig> {
    vec![
        MirrorConfig {
            name: "tsinghua".to_string(),
            priority: 1,
            base_url: "https://mirrors.tuna.tsinghua.edu.cn/Adoptium".to_string(),
            url_template: "{base_url}/{major}/jdk/{arch}/{os}/{filename}".to_string(),
            enabled: true,
        },
        MirrorConfig {
            name: "github".to_string(),
            priority: 2,
            base_url: String::new(),
            url_template: "https://github.com/adoptium/temurin{major}-binaries/releases/download/{tag}/{filename}".to_string(),
            enabled: true,
        },
    ]
}

/// 配置文件结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub java_environments: Vec<JavaEnvironment>,
    #[serde(default)]
    pub maven_environments: Vec<MavenEnvironment>,
    #[serde(default)]
    pub cc_environments: Vec<CcEnvironment>,
    #[serde(default)]
    pub mirrors: MirrorsConfig,
    /// Java 版本注册表路径（可选，默认使用编译嵌入的版本）
    #[serde(default)]
    pub java_versions_path: Option<String>,
    /// 下载配置
    #[serde(default)]
    pub download: DownloadConfig,
    /// 当前激活的 Java 环境名称
    #[serde(default)]
    pub current_java_env: Option<String>,
    /// 默认 Java 环境名称（类似 fnm 的默认版本）
    #[serde(default)]
    pub default_java_env: Option<String>,
    /// 当前激活的 Maven 环境名称
    #[serde(default)]
    pub current_maven_env: Option<String>,
    /// 默认 Maven 环境名称
    #[serde(default)]
    pub default_maven_env: Option<String>,
    #[serde(default)]
    pub default_cc_env: Option<String>,
    /// 自定义 Java 扫描路径
    #[serde(default)]
    pub custom_java_scan_paths: Vec<String>,
    /// 自定义 Maven 扫描路径
    #[serde(default)]
    pub custom_maven_scan_paths: Vec<String>,
    /// 明确移除的 Java 环境名称（防止重新扫描添加）
    #[serde(default)]
    pub removed_java_names: Vec<String>,
}

/// 下载配置
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DownloadConfig {
    /// 重试次数
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    /// 初始重试延迟（毫秒）
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
    /// 是否使用指数退避
    #[serde(default = "default_exponential_backoff")]
    pub exponential_backoff: bool,
    /// 连接超时时间（秒）
    #[serde(default = "default_connect_timeout_sec")]
    pub connect_timeout_sec: u64,
    /// 读取超时时间（秒）
    #[serde(default = "default_read_timeout_sec")]
    pub read_timeout_sec: u64,
}

fn default_retry_count() -> u32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

fn default_exponential_backoff() -> bool {
    true
}

fn default_connect_timeout_sec() -> u64 {
    30
}

fn default_read_timeout_sec() -> u64 {
    300
}

/// 默认 CC 环境配置（仅保留一个 anthropic-cc 作为初始示例）
fn default_cc_environments() -> Vec<CcEnvironment> {
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

/// 环境来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum EnvironmentSource {
    #[serde(rename = "manual")]
    #[default]
    Manual,
    #[serde(rename = "scanned")]
    Scanned,
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

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    /// 创建默认配置
    pub fn new() -> Self {
        Config {
            java_environments: Vec::new(),
            maven_environments: Vec::new(),
            cc_environments: default_cc_environments(),
            mirrors: MirrorsConfig::default(),
            download: DownloadConfig::default(),
            java_versions_path: None,
            current_java_env: None,
            default_java_env: None,
            current_maven_env: None,
            default_maven_env: None,
            default_cc_env: Some("anthropic-cc".to_string()),
            custom_java_scan_paths: Vec::new(),
            custom_maven_scan_paths: Vec::new(),
            removed_java_names: Vec::new(),
        }
    }

    /// 从文件加载配置
    pub fn load() -> Result<Self, String> {
        crate::infrastructure::paths::migrate_layout();
        let config_path = get_config_path()?;

        if !config_path.exists() {
            // 如果配置文件不存在，创建默认配置
            let config = Config::new();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {e}"))
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), String> {
        let config_path = get_config_path()?;

        // 确保配置目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let toml_content =
            toml::to_string_pretty(self).map_err(|e| format!("Failed to serialize config: {e}"))?;

        fs::write(&config_path, toml_content)
            .map_err(|e| format!("Failed to write config file: {e}"))?;

        Ok(())
    }

    /// 添加 Java 环境
    pub fn add_java_env(&mut self, env: JavaEnvironment) -> Result<(), String> {
        // 检查名称是否已存在
        if self.java_environments.iter().any(|e| e.name == env.name) {
            return Err(format!("Java environment '{}' already exists", env.name));
        }
        self.java_environments.push(env);
        Ok(())
    }

    /// 删除 Java 环境
    pub fn remove_java_env(&mut self, name: &str) -> Result<(), String> {
        let original_len = self.java_environments.len();
        self.java_environments.retain(|e| e.name != name);
        if self.java_environments.len() == original_len {
            return Err(format!("Java environment '{name}' does not exist"));
        }
        Ok(())
    }

    /// 获取 Java 环境
    pub fn get_java_env(&self, name: &str) -> Option<&JavaEnvironment> {
        self.java_environments.iter().find(|e| e.name == name)
    }

    /// 设置当前激活的 Java 环境
    pub fn set_current_java_env(&mut self, name: String) -> Result<(), String> {
        // 验证环境是否存在
        if !self.java_environments.iter().any(|e| e.name == name) {
            return Err(format!("Java environment '{name}' does not exist"));
        }
        self.current_java_env = Some(name);
        Ok(())
    }

    /// 获取当前激活的 Java 环境
    pub fn get_current_java_env(&self) -> Option<&JavaEnvironment> {
        if let Some(ref name) = self.current_java_env {
            self.get_java_env(name)
        } else {
            None
        }
    }

    /// 清除当前激活的 Java 环境
    pub fn clear_current_java_env(&mut self) {
        self.current_java_env = None;
    }

    /// 设置默认 Java 环境
    pub fn set_default_java_env(&mut self, name: String) -> Result<(), String> {
        // 跳过验证，直接设置默认环境
        self.default_java_env = Some(name);
        Ok(())
    }

    pub fn set_default_cc_env(&mut self, name: String) -> Result<(), String> {
        self.default_cc_env = Some(name);
        Ok(())
    }

    /// 获取默认 Java 环境
    pub fn get_default_java_env(&self) -> Option<&JavaEnvironment> {
        if let Some(ref name) = self.default_java_env {
            self.get_java_env(name)
        } else {
            None
        }
    }

    /// 清除默认 Java 环境
    pub fn clear_default_java_env(&mut self) {
        self.default_java_env = None;
    }

    /// 添加 Maven 环境
    pub fn add_maven_env(&mut self, env: MavenEnvironment) -> Result<(), String> {
        if self.maven_environments.iter().any(|e| e.name == env.name) {
            return Err(format!("Maven environment '{}' already exists", env.name));
        }
        self.maven_environments.push(env);
        Ok(())
    }

    /// 删除 Maven 环境
    pub fn remove_maven_env(&mut self, name: &str) -> Result<(), String> {
        let original_len = self.maven_environments.len();
        self.maven_environments.retain(|e| e.name != name);
        if self.maven_environments.len() == original_len {
            return Err(format!("Maven environment '{name}' does not exist"));
        }
        Ok(())
    }

    /// 获取 Maven 环境
    pub fn get_maven_env(&self, name: &str) -> Option<&MavenEnvironment> {
        self.maven_environments.iter().find(|e| e.name == name)
    }

    /// 设置当前激活的 Maven 环境
    pub fn set_current_maven_env(&mut self, name: String) -> Result<(), String> {
        if !self.maven_environments.iter().any(|e| e.name == name) {
            return Err(format!("Maven environment '{name}' does not exist"));
        }
        self.current_maven_env = Some(name);
        Ok(())
    }

    /// 获取当前激活的 Maven 环境
    pub fn get_current_maven_env(&self) -> Option<&MavenEnvironment> {
        if let Some(ref name) = self.current_maven_env {
            self.get_maven_env(name)
        } else {
            None
        }
    }

    /// 清除当前激活的 Maven 环境
    pub fn clear_current_maven_env(&mut self) {
        self.current_maven_env = None;
    }

    /// 设置默认 Maven 环境
    pub fn set_default_maven_env(&mut self, name: String) -> Result<(), String> {
        self.default_maven_env = Some(name);
        Ok(())
    }

    /// 获取默认 Maven 环境
    pub fn get_default_maven_env(&self) -> Option<&MavenEnvironment> {
        if let Some(ref name) = self.default_maven_env {
            self.get_maven_env(name)
        } else {
            None
        }
    }

    /// 清除默认 Maven 环境
    pub fn clear_default_maven_env(&mut self) {
        self.default_maven_env = None;
    }

    /// 获取有效的 Maven 环境（优先级：当前环境 → 默认环境）
    pub fn get_effective_maven_env(&self) -> Option<&MavenEnvironment> {
        if let Some(ref name) = self.current_maven_env {
            if let Some(env) = self.get_maven_env(name) {
                return Some(env);
            }
        }
        if let Some(ref name) = self.default_maven_env {
            self.get_maven_env(name)
        } else {
            None
        }
    }

    pub fn clear_default_cc_env(&mut self) {
        self.default_cc_env = None;
    }

    /// 获取有效的 Java 环境（优先级：当前环境 → 默认环境）
    pub fn get_effective_java_env(&self) -> Option<&JavaEnvironment> {
        // 首先尝试获取当前环境
        if let Some(ref name) = self.current_java_env {
            if let Some(env) = self.get_java_env(name) {
                return Some(env);
            }
        }

        // 如果没有当前环境，尝试获取默认环境
        if let Some(ref name) = self.default_java_env {
            self.get_java_env(name)
        } else {
            None
        }
    }

    /// 添加移除的 Java 环境名称
    pub fn add_removed_java_name(&mut self, name: &str) {
        if !self.removed_java_names.contains(&name.to_string()) {
            self.removed_java_names.push(name.to_string());
        }
    }

    /// 检查 Java 环境名称是否已被移除
    pub fn is_java_name_removed(&self, name: &str) -> bool {
        self.removed_java_names.contains(&name.to_string())
    }

    /// 移除 Java 环境名称（从移除列表中恢复）
    pub fn remove_java_name_from_removed_list(&mut self, name: &str) {
        self.removed_java_names.retain(|n| n != name);
    }

    /// 补全配置文件并写回，返回是否有变更被写入
    pub fn sync() -> Result<bool, String> {
        let config_path = get_config_path()?;
        let existed = config_path.exists();

        let mut config = if existed {
            // 如果配置文件存在，加载现有配置
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Failed to read config file: {e}"))?;
            toml::from_str(&content).map_err(|e| format!("Failed to parse config file: {e}"))?
        } else {
            // 如果配置文件不存在，创建默认配置
            Config::new()
        };

        // 智能补全缺失的 CC 环境配置
        let default_cc_envs = default_cc_environments();
        let mut updated = false;

        // 添加缺失的默认 CC 环境
        for default_env in default_cc_envs {
            if !config
                .cc_environments
                .iter()
                .any(|env| env.name == default_env.name)
            {
                config.cc_environments.push(default_env);
                updated = true;
            }
        }

        // 如果没有默认 CC 环境，设置一个
        if config.default_cc_env.is_none() && !config.cc_environments.is_empty() {
            config.default_cc_env = Some("anthropic-cc".to_string());
            updated = true;
        }

        // 补全下载配置为默认值（清华源）
        let default_config = Config::new();

        // 补全 mirrors 配置
        if config.mirrors.java.is_empty() {
            config.mirrors = default_config.mirrors.clone();
            updated = true;
        }
        if config.mirrors.maven.is_empty() {
            config.mirrors.maven = default_config.mirrors.maven.clone();
            updated = true;
        }

        // 序列化配置
        let serialized = toml::to_string_pretty(&config)
            .map_err(|e| format!("Failed to serialize config: {e}"))?;

        // 检查是否有变更
        if existed {
            if let Ok(current) = fs::read_to_string(&config_path) {
                if current == serialized && !updated {
                    return Ok(false);
                }
            }
        }

        // 保存配置
        config.save()?;
        Ok(true)
    }
}

/// 解析环境变量引用（如 ${VAR_NAME}）
pub fn resolve_env_var(value: &str) -> String {
    if value.starts_with("${") && value.ends_with('}') {
        let var_name = &value[2..value.len() - 1];
        env::var(var_name).unwrap_or_else(|_| value.to_string())
    } else {
        value.to_string()
    }
}

/// 获取配置文件路径
pub fn get_config_path() -> Result<PathBuf, String> {
    crate::infrastructure::paths::config_path()
}

/// 获取配置目录
pub fn get_config_dir() -> Result<PathBuf, String> {
    crate::infrastructure::paths::fnva_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_env_var() {
        // 设置测试环境变量
        env::set_var("TEST_VAR", "test_value");

        let resolved = resolve_env_var("${TEST_VAR}");
        assert_eq!(resolved, "test_value");

        let not_resolved = resolve_env_var("normal_value");
        assert_eq!(not_resolved, "normal_value");

        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_config_add_java_env() {
        let mut config = Config::new();
        let env = JavaEnvironment {
            name: "test".to_string(),
            java_home: "/usr/lib/jvm/java-17".to_string(),
            description: "Test JDK".to_string(),
            source: EnvironmentSource::Manual,
        };

        assert!(config.add_java_env(env.clone()).is_ok());
        assert!(config.add_java_env(env).is_err()); // 重复添加应该失败
    }

    #[test]
    fn test_config_add_maven_env() {
        let mut config = Config::new();
        let env = MavenEnvironment {
            name: "maven3".to_string(),
            maven_home: "/home/user/.fnva/packages/maven/3.9.16".to_string(),
            description: "Maven 3.9.16".to_string(),
            source: EnvironmentSource::Manual,
            maven_opts: None,
            local_repo: None,
            settings_file: None,
        };
        assert!(config.add_maven_env(env.clone()).is_ok());
        assert!(config.add_maven_env(env).is_err()); // 重复添加应该失败
        assert_eq!(
            config.get_maven_env("maven3").unwrap().maven_home,
            "/home/user/.fnva/packages/maven/3.9.16"
        );
    }

    #[test]
    fn test_default_maven_mirrors() {
        let m = default_maven_mirrors();
        assert!(m.iter().any(|x| x.name == "tsinghua"));
        assert!(m.iter().any(|x| x.name == "apache-archive"));
        // 清华优先级最高
        assert_eq!(
            m.iter().min_by_key(|x| x.priority).unwrap().name,
            "tsinghua"
        );
    }

    #[test]
    fn test_config_maven_toml_roundtrip() {
        let mut config = Config::new();
        config
            .add_maven_env(MavenEnvironment {
                name: "mvn39".to_string(),
                maven_home: "/x/maven".to_string(),
                description: "M".to_string(),
                source: EnvironmentSource::Manual,
                maven_opts: None,
                local_repo: None,
                settings_file: None,
            })
            .unwrap();
        let toml_str = toml::to_string_pretty(&config).expect("serialize");
        let parsed: Config = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(
            parsed.get_maven_env("mvn39").unwrap().maven_home,
            "/x/maven"
        );
        // maven 镜像默认补全
        assert!(!parsed.mirrors.maven.is_empty());
    }
}
