use crate::core::environment_manager::{DynEnvironment, EnvironmentManager, EnvironmentType};
use crate::core::presentation::ScanHit;
use crate::core::session::SessionManager;
use crate::error::AppError;
use crate::infrastructure::config::{
    CcEnvironment as ConfigCcEnvironment, Config, DEFAULT_SONNET_MODEL,
};
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
    fn load_from_config(&mut self) -> Result<(), AppError> {
        let config = Config::load().map_err(|e| AppError::config_error(&e))?;

        self.environments.clear();
        for env in &config.cc_environments {
            let cc_env = ConfigCcEnvironment {
                name: env.name.clone(),
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

    fn list(&self) -> Result<Vec<DynEnvironment>, AppError> {
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

    fn get(&self, name: &str) -> Result<Option<DynEnvironment>, AppError> {
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

    fn add(&mut self, name: &str, config_str: &str) -> Result<(), AppError> {
        // Parse config as JSON
        let config: serde_json::Value = serde_json::from_str(config_str)?;

        let api_key = config.get("api_key").and_then(|v| v.as_str()).unwrap_or("");

        let base_url = config
            .get("base_url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("base_url", "missing in config"))?;

        // Support both "model" (legacy) and "sonnet_model" (new)
        let sonnet_model = config
            .get("sonnet_model")
            .or_else(|| config.get("model"))
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_SONNET_MODEL);

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
        let mut file_config = Config::load().map_err(|e| AppError::config_error(&e))?;
        if let Some(existing) = file_config
            .cc_environments
            .iter_mut()
            .find(|env| env.name == name)
        {
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
        file_config.save().map_err(|e| AppError::config_error(&e))?;

        self.environments.insert(name.to_string(), cc_environment);
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), AppError> {
        if self.environments.remove(name).is_none() {
            return Err(AppError::not_found(&format!("CC environment '{name}'")));
        }

        let mut config = Config::load().map_err(|e| AppError::config_error(&e))?;
        let original_len = config.cc_environments.len();
        config.cc_environments.retain(|env| env.name != name);
        if config.cc_environments.len() == original_len {
            return Err(AppError::not_found(&format!("CC environment '{name}'")));
        }

        config.save().map_err(|e| AppError::config_error(&e))?;

        Ok(())
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, AppError> {
        let cc_env = self
            .environments
            .get(name)
            .ok_or_else(|| AppError::not_found(&format!("CC environment '{name}'")))?;

        let shell_type =
            shell_type.unwrap_or_else(crate::infrastructure::shell::platform::detect_shell);

        // Create config for script generation
        let mut config = serde_json::json!({
            "api_key": cc_env.api_key,
            "base_url": cc_env.base_url,
            "sonnet_model": cc_env.sonnet_model,
        });

        // Inject Anthropic-protocol vars (ANTHROPIC_AUTH_TOKEN / BASE_URL / models)
        crate::environments::cc::setup::apply_anthropic_config(cc_env, &mut config);

        let generator = ScriptGenerator::new()?;
        generator.generate_switch_script(EnvironmentType::Cc, name, &config, Some(shell_type))
    }

    fn get_current(&self) -> Result<Option<String>, AppError> {
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

    async fn scan(&self, _extra: &[String]) -> Result<Vec<ScanHit>, AppError> {
        let mut result: Vec<ScanHit> = Vec::new();
        let existing_config = crate::infrastructure::config::Config::load().unwrap_or_default();
        let existing_names: std::collections::HashSet<String> = existing_config
            .cc_environments
            .iter()
            .map(|e| e.name.clone())
            .collect();

        // Cross-platform home dir for "~" shortening. `HOME` is unset on
        // Windows, so `std::env::var("HOME").unwrap_or_default()` yields "" and
        // `String::replace("", "~")` would insert "~" between every character —
        // garbling the whole path (e.g. ~C~:~\~U~s~e~r~s~...). `dirs::home_dir()`
        // resolves `%USERPROFILE%` on Windows and `$HOME` on Unix.
        let home_str = dirs::home_dir()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_default();

        // 1. Scan Claude Code settings.json across all supported platforms
        for candidate in claude_settings_candidates() {
            if !candidate.exists() {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&candidate) else {
                continue;
            };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) else {
                continue;
            };
            let Some(env_map) = json.get("env").and_then(|v| v.as_object()) else {
                continue;
            };

            let base_url = env_map
                .get("ANTHROPIC_BASE_URL")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            if base_url.is_empty() {
                continue;
            }

            let name = url_to_env_name(&base_url);
            if existing_names.contains(&name)
                || existing_config
                    .cc_environments
                    .iter()
                    .any(|e| e.base_url == base_url)
                || result.iter().any(|e: &ScanHit| e.location == base_url)
            {
                continue;
            }

            let sonnet_model = env_map
                .get("ANTHROPIC_DEFAULT_SONNET_MODEL")
                .and_then(|v| v.as_str())
                .unwrap_or(DEFAULT_SONNET_MODEL)
                .to_string();

            let source_label = if home_str.is_empty() {
                candidate.to_string_lossy().into_owned()
            } else {
                candidate.to_string_lossy().replace(&home_str, "~")
            };
            let import = format!(
                "fnva cc add --name {name} --base-url \"{base_url}\""
            );
            result.push(ScanHit {
                name,
                location: base_url,
                detail: format!("{sonnet_model} (from {source_label})"),
                import_cmd: import,
            });
        }

        // 2. Check system environment variables
        if let (Ok(_auth_token), Ok(base_url)) = (
            std::env::var("ANTHROPIC_AUTH_TOKEN"),
            std::env::var("ANTHROPIC_BASE_URL"),
        ) {
            let name = url_to_env_name(&base_url);
            let already_in_result = result.iter().any(|e| e.location == base_url);
            let already_managed = existing_config
                .cc_environments
                .iter()
                .any(|e| e.base_url == base_url);

            if !already_in_result && !already_managed {
                let sonnet = std::env::var("ANTHROPIC_DEFAULT_SONNET_MODEL")
                    .unwrap_or_else(|_| DEFAULT_SONNET_MODEL.to_string());
                let import = format!(
                    "fnva cc add --name {name} --base-url \"{base_url}\""
                );
                result.push(ScanHit {
                    name,
                    location: base_url,
                    detail: format!("{sonnet} (from env vars)"),
                    import_cmd: import,
                });
            }
        }

        Ok(result)
    }

    fn set_current(&mut self, _name: &str) -> Result<(), AppError> {
        // This would set the current environment by updating environment variables
        // For now, this is a no-op - the actual switching is handled by use_env
        Ok(())
    }

    fn is_available(&self, name: &str) -> Result<bool, AppError> {
        Ok(self.environments.contains_key(name))
    }

    fn get_details(&self, name: &str) -> Result<Option<DynEnvironment>, AppError> {
        self.get(name)
    }
}

/// Returns all candidate paths for Claude Code's settings.json on the current platform.
///
/// Claude Code (CLI) uses `~/.claude/settings.json` on **all** platforms.
/// Note: `%APPDATA%\Claude\` is used by Claude Desktop (not Claude Code).
///
/// | Platform | Path |
/// |----------|------|
/// | Linux    | `~/.claude/settings.json` |
/// | macOS    | `~/.claude/settings.json` |
/// | Windows  | `%USERPROFILE%\.claude\settings.json` (same as ~/.claude) |
fn claude_settings_candidates() -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::new();

    // All platforms: ~/.claude/settings.json
    // On Windows this resolves to %USERPROFILE%\.claude\settings.json
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".claude").join("settings.json"));
    }

    // macOS additional location: ~/Library/Application Support/Claude/settings.json
    #[cfg(target_os = "macos")]
    if let Some(app_support) = dirs::data_dir() {
        candidates.push(app_support.join("Claude").join("settings.json"));
    }

    candidates
}

/// Convert a base URL to a meaningful short env name.
/// e.g. "https://open.bigmodel.cn/api/anthropic" → "bigmodel-cc"
///      "https://api.anthropic.com"              → "anthropic-cc"
fn url_to_env_name(base_url: &str) -> String {
    // Strip scheme
    let without_scheme = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    // Take only the hostname portion
    let host = without_scheme.split('/').next().unwrap_or(without_scheme);
    // Take the second-level domain segment (e.g. "bigmodel" from "open.bigmodel.cn")
    let parts: Vec<&str> = host.split('.').collect();
    let label = if parts.len() >= 2 {
        parts[parts.len() - 2]
    } else {
        parts.first().copied().unwrap_or("cc")
    };
    format!("{label}-cc")
}

// 为 ConfigCcEnvironment 添加扩展方法
impl ConfigCcEnvironment {
    fn is_active(&self) -> bool {
        crate::environments::cc::setup::is_anthropic_active()
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
