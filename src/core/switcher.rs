use crate::core::environment_manager::{EnvironmentManager, EnvironmentType, SwitchResult};
use crate::core::presentation::{EnvItem, HistoryItem, OutputFormat};
use crate::core::session::{HistoryManager, SessionManager, SwitchHistory};
use crate::error::{
    option_with_context, safe_to_json, safe_to_json_pretty, AppError, AppResult, ResultExt,
    SafeMutex,
};
use crate::infrastructure::config::Config;
use crate::infrastructure::shell::current_envs::CurrentEnvsFile;
use crate::infrastructure::shell::{script_factory::ScriptGenerator, ShellType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// 环境切换器
pub struct EnvironmentSwitcher {
    /// 环境管理器
    managers: HashMap<EnvironmentType, Arc<Mutex<dyn EnvironmentManager>>>,
    /// 会话管理器
    session_manager: SafeMutex<SessionManager>,
    /// 历史管理器
    history_manager: SafeMutex<HistoryManager>,
}

impl EnvironmentSwitcher {
    /// 创建新的环境切换器
    pub fn new() -> AppResult<Self> {
        let session_manager = SessionManager::new().map_err(|e| AppError::Config {
            message: format!("Failed to create session manager: {e}"),
        })?;
        let history_manager = HistoryManager::new(100).map_err(|e| AppError::Internal {
            message: format!("Failed to create history manager: {e}"),
        })?;

        Ok(Self {
            managers: HashMap::new(),
            session_manager: SafeMutex::new(session_manager, "session_manager"),
            history_manager: SafeMutex::new(history_manager, "history_manager"),
        })
    }

    pub fn register_manager(
        &mut self,
        env_type: EnvironmentType,
        manager: Arc<Mutex<dyn EnvironmentManager>>,
    ) -> AppResult<()> {
        self.managers.insert(env_type, manager);
        Ok(())
    }

    /// 切换环境
    pub async fn switch_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
        shell_type: Option<ShellType>,
        reason: Option<String>,
    ) -> AppResult<SwitchResult> {
        // 获取环境管理器
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when switching environment",
        )?;

        // 获取当前环境（不可变）
        let old_env = {
            let manager_guard = manager.lock().await;
            manager_guard
                .get_current()
                .with_context("getting current environment")?
        };

        // 验证环境是否存在
        let env_info = {
            let manager_guard = manager.lock().await;
            manager_guard
                .get(name)
                .with_context(&format!("looking up {env_type} environment '{name}'"))?
        };

        if env_info.is_none() {
            return Ok(SwitchResult {
                name: name.to_string(),
                env_type,
                script: String::new(),
                success: false,
                error: Some(format!("Environment '{name}' not found")),
            });
        }

        // 生成切换脚本（需要可变借用）
        let script = {
            let mut manager_guard = manager.lock().await;
            manager_guard
                .use_env(name, shell_type)
                .with_context(&format!("switching to {env_type} environment '{name}'"))?
        };

        // 更新会话状态
        {
            let mut session_manager = self.session_manager.lock()?;
            session_manager
                .set_current_environment(env_type, name)
                .map_err(|e| AppError::Config {
                    message: format!("Failed to update session state: {e}"),
                })?;
        }

        // Persist to current_envs.toml for shell hook auto-restore
        {
            if let Err(e) = CurrentEnvsFile::write(env_type, name) {
                eprintln!("⚠  Failed to update current_envs.toml: {e}");
            }
        }

        // 记录历史
        {
            let mut history_manager = self.history_manager.lock()?;
            history_manager
                .record_switch(env_type, old_env, name.to_string(), reason)
                .map_err(|e| AppError::Internal {
                    message: format!("Failed to record switch history: {e}"),
                })?;
        }

        Ok(SwitchResult {
            name: name.to_string(),
            env_type,
            script,
            success: true,
            error: None,
        })
    }

    /// 添加环境
    pub async fn add_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
        config: serde_json::Value,
    ) -> AppResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when adding environment",
        )?;

        // 配置由各 manager 的 add() 按环境类型解析与校验,这里只透传 JSON 字符串。
        let result = {
            let mut manager_guard = manager.lock().await;

            // Convert JSON Value to string for the object-safe interface
            let config_str = safe_to_json(&config)?;

            manager_guard
                .add(name, &config_str)
                .with_context(&format!("adding {env_type} environment '{name}'"))?;

            format!("Added {env_type} environment: {name}")
        };

        Ok(result)
    }

    /// 删除环境
    pub async fn remove_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
    ) -> AppResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when removing environment",
        )?;

        {
            let mut manager_guard = manager.lock().await;
            manager_guard
                .remove(name)
                .with_context(&format!("removing {env_type} environment '{name}'"))?;
        }

        // 如果删除的是当前环境，清除会话状态
        {
            let mut session_manager = self.session_manager.lock()?;
            if let Some(current) = session_manager.get_current_environment(env_type) {
                if current == name {
                    session_manager
                        .remove_current_environment(env_type)
                        .map_err(|e| AppError::Config {
                            message: format!("Failed to clear current environment: {e}"),
                        })?;
                    if let Err(e) = CurrentEnvsFile::clear(env_type) {
                        eprintln!("⚠  Failed to clear current_envs.toml: {e}");
                    }
                }
            }
        }

        Ok(format!(
            "Successfully removed {env_type} environment: {name}"
        ))
    }

    /// 获取当前环境
    pub async fn get_current_environment(
        &self,
        env_type: EnvironmentType,
        output_format: OutputFormat,
    ) -> AppResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when getting current environment",
        )?;

        let (current_env, manager_guard) = {
            let manager_guard = manager.lock().await;
            let current_env = manager_guard
                .get_current()
                .with_context("getting current environment")?;
            (current_env, manager_guard)
        };

        match output_format {
            OutputFormat::Text => {
                if let Some(env_name) = current_env {
                    if let Some(env_info) = manager_guard
                        .get(&env_name)
                        .with_context("getting environment info")?
                    {
                        Ok(format!(
                            "Current {} environment: {}\n{}\n",
                            env_type,
                            env_name,
                            env_info.description.clone().unwrap_or_default()
                        ))
                    } else {
                        Ok(format!(
                            "Current {env_type} environment: {env_name} (details unavailable)\n"
                        ))
                    }
                } else {
                    Ok(format!("No current {env_type} environment\n"))
                }
            }
            OutputFormat::Json => {
                let json_output = if let Some(env_name) = current_env {
                    if let Some(env_info) = manager_guard
                        .get(&env_name)
                        .with_context("getting environment info")?
                    {
                        serde_json::json!({
                            "environment_type": env_type,
                            "name": env_name,
                            "details": env_info
                        })
                    } else {
                        serde_json::json!({
                            "environment_type": env_type,
                            "name": env_name,
                            "details": null
                        })
                    }
                } else {
                    serde_json::json!({
                        "environment_type": env_type,
                        "name": null,
                        "details": null
                    })
                };
                Ok(safe_to_json_pretty(&json_output)?)
            }
        }
    }

    /// 生成 shell 集成脚本
    pub async fn generate_shell_integration(&self, shell_type: ShellType) -> AppResult<String> {
        let current_envs = self.session_manager.lock()?.get_all_current().clone();

        let generator = ScriptGenerator::new().map_err(|e| AppError::ScriptGeneration {
            shell_type: format!("{shell_type:?}"),
            reason: e.to_string(),
        })?;

        generator.generate_integration_script(&current_envs, Some(shell_type))
    }

    /// 扫描环境
    pub async fn scan_environments(
        &self,
        env_type: EnvironmentType,
        extra: &[String],
    ) -> AppResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when scanning environments",
        )?;

        let found_envs = {
            let manager_guard = manager.lock().await;
            manager_guard
                .scan(extra)
                .await
                .with_context(&format!("scanning {env_type} environments"))?
        };

        let mut output = String::new();
        if found_envs.is_empty() {
            output.push_str(&format!("No new {env_type} environments found on system\n"));
            output.push_str("(Tip: already-managed environments are excluded; use --path to scan custom locations)\n");
        } else {
            output.push_str(&format!(
                "Found {} new {env_type} environment(s):\n\n",
                found_envs.len(),
            ));
            for h in &found_envs {
                output.push_str(&format!("  Name    : {}\n", h.name));
                output.push_str(&format!("  Location: {}\n", h.location));
                if !h.detail.is_empty() {
                    output.push_str(&format!("  Detail  : {}\n", h.detail));
                }
                output.push_str(&format!("  Import  : {}\n\n", h.import_cmd));
            }
        }

        Ok(output)
    }

    /// 获取切换历史
    pub async fn get_switch_history(
        &self,
        env_type: Option<EnvironmentType>,
        limit: usize,
    ) -> AppResult<Vec<HistoryItem>> {
        let history: Vec<SwitchHistory> = {
            let history_manager = self.history_manager.lock()?;

            if let Some(env_type) = env_type {
                history_manager
                    .get_history_for_env(env_type)
                    .into_iter()
                    .rev()
                    .take(limit)
                    .cloned()
                    .collect()
            } else {
                history_manager
                    .get_recent_history(limit)
                    .into_iter()
                    .rev()
                    .cloned()
                    .collect()
            }
        };

        let mut items = Vec::new();
        for record in history {
            items.push(HistoryItem {
                timestamp: record.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                env_type: format!("{}", record.env_type),
                from: record.old_env.clone(),
                to: record.new_env.clone(),
            });
        }
        Ok(items)
    }

    /// 设置默认环境
    pub async fn set_default_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
    ) -> AppResult<String> {
        let manager_entry = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when setting default environment",
        )?;

        {
            let manager = manager_entry.lock().await;
            if !manager
                .is_available(name)
                .with_context("checking environment availability")?
            {
                return Err(AppError::Environment {
                    message: format!("{env_type} environment '{name}' not found"),
                });
            }
        }

        let mut config = Config::load().map_err(|e| AppError::Config {
            message: format!("Failed to load config: {e}"),
        })?;

        match env_type {
            EnvironmentType::Java => {
                config
                    .set_default_java_env(name.to_string())
                    .map_err(|e| AppError::Config {
                        message: format!("Failed to set default Java environment: {e}"),
                    })?
            }
            EnvironmentType::Cc => {
                config
                    .set_default_cc_env(name.to_string())
                    .map_err(|e| AppError::Config {
                        message: format!("Failed to set default CC environment: {e}"),
                    })?
            }
            EnvironmentType::Maven => {
                config
                    .set_default_maven_env(name.to_string())
                    .map_err(|e| AppError::Config {
                        message: format!("Failed to set default Maven environment: {e}"),
                    })?
            }
        }

        config.save().map_err(|e| AppError::Config {
            message: format!("Failed to save config: {e}"),
        })?;

        Ok(format!("Set default {env_type} environment: {name}"))
    }

    /// 清除默认环境
    pub async fn clear_default_environment(&self, env_type: EnvironmentType) -> AppResult<String> {
        let mut config = Config::load().map_err(|e| AppError::Config {
            message: format!("Failed to load config: {e}"),
        })?;

        match env_type {
            EnvironmentType::Java => config.clear_default_java_env(),
            EnvironmentType::Cc => config.clear_default_cc_env(),
            EnvironmentType::Maven => config.clear_default_maven_env(),
        }

        config.save().map_err(|e| AppError::Config {
            message: format!("Failed to save config: {e}"),
        })?;

        Ok(format!("Cleared default {env_type} environment"))
    }

    /// 获取默认环境
    pub async fn get_default_environment(
        &self,
        env_type: EnvironmentType,
    ) -> AppResult<Option<String>> {
        let config = Config::load().map_err(|e| AppError::Config {
            message: format!("Failed to load config: {e}"),
        })?;

        let default_env = match env_type {
            EnvironmentType::Java => config.default_java_env.clone(),
            EnvironmentType::Cc => config.default_cc_env.clone(),
            EnvironmentType::Maven => config.default_maven_env.clone(),
        };
        Ok(default_env)
    }

    /// 列出环境时显示默认环境标记
    pub async fn list_environments_with_default(
        &self,
        env_type: EnvironmentType,
    ) -> AppResult<Vec<EnvItem>> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when listing environments",
        )?;

        // 一次性加载配置，避免重复读取
        let config = Config::load().map_err(|e| AppError::Config {
            message: format!("Failed to load config: {e}"),
        })?;

        let (environments, current_env) = {
            let manager_guard = manager.lock().await;
            let environments = manager_guard.list().with_context("listing environments")?;
            let current_env = {
                let session_manager = self.session_manager.lock()?;
                session_manager.get_current_environment(env_type).cloned()
            };
            (environments, current_env)
        };
        let default_env = match env_type {
            EnvironmentType::Java => config.default_java_env.clone(),
            EnvironmentType::Cc => config.default_cc_env.clone(),
            EnvironmentType::Maven => config.default_maven_env.clone(),
        };

        let mut items = Vec::new();
        for env in environments {
            let name = env.name.clone();
            let is_current = current_env.as_ref() == Some(&name);
            let is_default = default_env.as_ref() == Some(&name);
            // CC 环境把模型显示在 extra
            let extra = if env_type == EnvironmentType::Cc {
                env.version.filter(|m| !m.is_empty())
            } else {
                None
            };
            // CC 缺 api_key 时标记,提醒该环境导出后无法鉴权
            let missing_key = if env_type == EnvironmentType::Cc {
                config
                    .cc_environments
                    .iter()
                    .find(|e| e.name == name)
                    .map(|e| e.api_key.trim().is_empty())
                    .unwrap_or(false)
            } else {
                false
            };
            items.push(EnvItem {
                name,
                description: env.description.clone().unwrap_or_default(),
                extra,
                is_current,
                is_default,
                missing_key,
            });
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environments::java::JavaEnvironmentManager;
    use crate::infrastructure::config::{Config, EnvironmentSource, JavaEnvironment};
    use std::sync::OnceLock;

    // 所有依赖 FNVA_HOME 的测试必须串行运行:环境变量是进程全局的,
    // cargo test 默认多线程并行会导致 set_var 互相覆盖。
    static SEQUENTIAL: OnceLock<std::sync::Mutex<()>> = OnceLock::new();

    /// 把 FNVA_HOME 指向给定临时目录,作用域结束时还原环境变量。
    struct FnvaHomeGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl FnvaHomeGuard {
        fn new(dir: &std::path::Path) -> Self {
            let lock = SEQUENTIAL
                .get_or_init(|| std::sync::Mutex::new(()))
                .lock()
                .unwrap();
            std::env::set_var("FNVA_HOME", dir);
            Self { _lock: lock }
        }
    }

    impl Drop for FnvaHomeGuard {
        fn drop(&mut self) {
            std::env::remove_var("FNVA_HOME");
        }
    }

    fn make_switcher() -> EnvironmentSwitcher {
        let mut switcher = EnvironmentSwitcher::new().expect("switcher init");
        let java = JavaEnvironmentManager::new();
        switcher
            .register_manager(EnvironmentType::Java, Arc::new(Mutex::new(java)))
            .expect("register java manager");
        switcher
    }

    #[tokio::test]
    async fn test_switch_unknown_environment_returns_failed_result() {
        let tmp = tempfile::TempDir::new().unwrap();
        let _guard = FnvaHomeGuard::new(tmp.path());

        let switcher = make_switcher();
        let result = switcher
            .switch_environment(EnvironmentType::Java, "does-not-exist", None, None)
            .await
            .expect("switch should resolve");

        // 不存在的环境返回 Ok 但标记失败(而非 Err),与 cli 层约定一致。
        assert!(!result.success);
        assert!(
            result
                .error
                .as_deref()
                .map(|e| e.contains("not found"))
                .unwrap_or(false),
            "error should mention 'not found', got: {:?}",
            result.error
        );
    }

    #[tokio::test]
    async fn test_list_environments_resolves_on_empty_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let _guard = FnvaHomeGuard::new(tmp.path());

        let switcher = make_switcher();
        // 空配置下列表应成功返回(覆盖 switcher 初始化 + list 路径)。
        switcher
            .list_environments_with_default(EnvironmentType::Java)
            .await
            .expect("list should resolve");
    }

    #[tokio::test]
    async fn test_switch_invalid_java_home_errors() {
        let tmp = tempfile::TempDir::new().unwrap();
        let _guard = FnvaHomeGuard::new(tmp.path());

        // 写入一个 java_home 指向无效路径的环境:load_from_config 不校验路径,
        // 因此能进入 installations,再由 use_env 的 validate_java_home 触发错误。
        {
            let mut config = Config::new();
            config
                .add_java_env(JavaEnvironment {
                    name: "bad".to_string(),
                    java_home: "/nonexistent/path/to/java".to_string(),
                    description: "test".to_string(),
                    source: EnvironmentSource::Manual,
                })
                .expect("add bad java env");
            config.save().expect("save config");
        }

        let switcher = make_switcher();
        let result = switcher
            .switch_environment(EnvironmentType::Java, "bad", None, None)
            .await;
        assert!(
            result.is_err(),
            "switching env with invalid java_home should error"
        );
    }

    #[tokio::test]
    async fn test_maven_default_roundtrip() {
        use crate::environments::maven::MavenEnvironmentManager;
        use crate::infrastructure::config::MavenEnvironment;

        let tmp = tempfile::TempDir::new().unwrap();
        let _guard = FnvaHomeGuard::new(tmp.path());

        // 配置一个 maven 环境(home 不校验,仅用于 default 闭环验证)
        {
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
                .expect("add maven env");
            config.save().expect("save config");
        }

        let mut switcher = EnvironmentSwitcher::new().expect("switcher init");
        switcher
            .register_manager(
                EnvironmentType::Maven,
                Arc::new(Mutex::new(MavenEnvironmentManager::new())),
            )
            .expect("register maven manager");

        // Maven default 之前只对 Java/CC 开放,这里验证三态闭环接通。
        switcher
            .set_default_environment(EnvironmentType::Maven, "mvn39")
            .await
            .expect("set maven default");
        assert_eq!(
            switcher
                .get_default_environment(EnvironmentType::Maven)
                .await
                .expect("get maven default"),
            Some("mvn39".to_string())
        );

        switcher
            .clear_default_environment(EnvironmentType::Maven)
            .await
            .expect("clear maven default");
        assert_eq!(
            switcher
                .get_default_environment(EnvironmentType::Maven)
                .await
                .expect("get maven default after clear"),
            None
        );
    }
}
