use crate::cli::output::OutputFormat;
use crate::core::environment_manager::{EnvironmentManager, EnvironmentType, SwitchResult};
use crate::core::session::{HistoryManager, SessionManager, SwitchHistory};
use crate::error::{
    option_with_context, safe_to_json, safe_to_json_pretty, AppError, ContextualResult, SafeMutex,
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
    pub fn new() -> ContextualResult<Self> {
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
    ) -> ContextualResult<()> {
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
    ) -> ContextualResult<SwitchResult> {
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
                .map_err(|e| AppError::Environment {
                    message: format!("Failed to get current environment: {e}"),
                })?
        };

        // 验证环境是否存在
        let env_info = {
            let manager_guard = manager.lock().await;
            manager_guard.get(name).map_err(|e| AppError::Environment {
                message: format!("Failed to find environment '{name}': {e}"),
            })?
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
                .map_err(|e| AppError::ScriptGeneration {
                    shell_type: format!("{:?}", shell_type.unwrap_or(ShellType::Bash)),
                    reason: e,
                })?
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
                crate::cli::print::warn(&format!("Failed to update current_envs.toml: {e}"));
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
    ) -> ContextualResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when adding environment",
        )?;

        // 这里需要根据环境类型解析配置
        // TODO: 实现配置解析逻辑

        let result = {
            let mut manager_guard = manager.lock().await;

            // Convert JSON Value to string for the object-safe interface
            let config_str = safe_to_json(&config)?;

            manager_guard
                .add(name, &config_str)
                .map_err(|e| AppError::Environment {
                    message: format!("Failed to add environment: {e}"),
                })?;

            format!("Successfully added {env_type} environment: {name}")
        };

        Ok(result)
    }

    /// 删除环境
    pub async fn remove_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
    ) -> ContextualResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when removing environment",
        )?;

        {
            let mut manager_guard = manager.lock().await;
            manager_guard
                .remove(name)
                .map_err(|e| AppError::Environment {
                    message: format!("Failed to remove environment: {e}"),
                })?;
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
                        crate::cli::print::warn(&format!("Failed to clear current_envs.toml: {e}"));
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
    ) -> ContextualResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when getting current environment",
        )?;

        let (current_env, manager_guard) = {
            let manager_guard = manager.lock().await;
            let current_env = manager_guard
                .get_current()
                .map_err(|e| AppError::Environment {
                    message: format!("Failed to get current environment: {e}"),
                })?;
            (current_env, manager_guard)
        };

        match output_format {
            OutputFormat::Text => {
                if let Some(env_name) = current_env {
                    if let Some(env_info) =
                        manager_guard
                            .get(&env_name)
                            .map_err(|e| AppError::Environment {
                                message: format!("Failed to get environment info: {e}"),
                            })?
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
                    if let Some(env_info) =
                        manager_guard
                            .get(&env_name)
                            .map_err(|e| AppError::Environment {
                                message: format!("Failed to get environment info: {e}"),
                            })?
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
    pub async fn generate_shell_integration(
        &self,
        shell_type: ShellType,
    ) -> ContextualResult<String> {
        let current_envs = self.session_manager.lock()?.get_all_current().clone();

        let generator = ScriptGenerator::new().map_err(|e| AppError::ScriptGeneration {
            shell_type: format!("{shell_type:?}"),
            reason: e.to_string(),
        })?;

        Ok(generator.generate_integration_script(&current_envs, Some(shell_type))?)
    }

    /// 扫描环境
    pub async fn scan_environments(&self, env_type: EnvironmentType) -> ContextualResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when scanning environments",
        )?;

        let found_envs = {
            let manager_guard = manager.lock().await;
            manager_guard
                .scan()
                .await
                .map_err(|e| AppError::Environment {
                    message: format!("Failed to scan environments: {e}"),
                })?
        };

        let mut output = String::new();
        if found_envs.is_empty() {
            output.push_str(&format!("No new {env_type} environments found on system\n"));
            output.push_str("(Tip: already-managed environments are excluded from scan results)\n");
        } else {
            output.push_str(&format!(
                "Found {} new {env_type} environment(s):\n\n",
                found_envs.len(),
            ));
            for env in &found_envs {
                let desc = env.description.as_deref().unwrap_or("-");
                let model = env.version.as_deref().unwrap_or("-");
                output.push_str(&format!("  Name    : {}\n", env.name));
                output.push_str(&format!("  URL     : {}\n", env.path));
                output.push_str(&format!("  Model   : {model}\n"));
                output.push_str(&format!("  Source  : {desc}\n"));
                // 生成与该环境类型对应的导入命令（不同类型的 add 子命令参数不同）
                let import_cmd = match env_type {
                    EnvironmentType::Java | EnvironmentType::Maven => {
                        format!(
                            "fnva {env_type} add --name {} --home \"{}\"",
                            env.name, env.path
                        )
                    }
                    EnvironmentType::Cc => {
                        // CC 的 add 必填 --provider，但扫描只能拿到 base_url，无法推断 provider，
                        // 故 --provider 留作占位符让用户补全。
                        format!(
                            "fnva cc add --name {} --provider <provider> --base-url \"{}\"",
                            env.name, env.path
                        )
                    }
                };
                output.push_str(&format!("  Import  : {import_cmd}\n\n"));
            }
        }

        Ok(output)
    }

    /// 获取切换历史
    pub async fn get_switch_history(
        &self,
        env_type: Option<EnvironmentType>,
        limit: usize,
    ) -> ContextualResult<String> {
        let history: Vec<SwitchHistory> = {
            let history_manager = self.history_manager.lock()?;

            if let Some(env_type) = env_type {
                // get_history_for_env returns Vec<&SwitchHistory>
                // We need to convert the references to owned values
                history_manager
                    .get_history_for_env(env_type)
                    .into_iter()
                    .rev()
                    .take(limit)
                    .cloned() // Clone to get owned SwitchHistory
                    .collect()
            } else {
                // get_recent_history returns Vec<&SwitchHistory>
                history_manager
                    .get_recent_history(limit)
                    .into_iter()
                    .rev()
                    .cloned()
                    .collect()
            }
        };

        use crate::cli::print::{format_history, HistoryItem};
        let mut items = Vec::new();
        for record in history {
            items.push(HistoryItem {
                timestamp: record.timestamp.format("%Y-%m-%d %H:%M").to_string(),
                env_type: format!("{}", record.env_type),
                from: record.old_env.clone(),
                to: record.new_env.clone(),
            });
        }
        Ok(format_history(&items))
    }

    /// 设置默认环境
    pub async fn set_default_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
    ) -> ContextualResult<String> {
        let manager_entry = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "finding environment manager when setting default environment",
        )?;

        {
            let manager = manager_entry.lock().await;
            if !manager
                .is_available(name)
                .map_err(|e| AppError::Environment {
                    message: format!("Failed to check environment availability: {e}"),
                })?
            {
                return Err(AppError::Environment {
                    message: format!("{env_type} environment '{name}' not found"),
                }
                .into());
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
            _ => {
                return Err(AppError::Validation {
                    field: "env_type".to_string(),
                    reason:
                        "Default environment support is currently only available for Java and CC"
                            .to_string(),
                }
                .into())
            }
        }

        config.save().map_err(|e| AppError::Config {
            message: format!("Failed to save config: {e}"),
        })?;

        Ok(format!("Set default {env_type} environment: {name}"))
    }

    /// 清除默认环境
    pub async fn clear_default_environment(
        &self,
        env_type: EnvironmentType,
    ) -> ContextualResult<String> {
        let mut config = Config::load().map_err(|e| AppError::Config {
            message: format!("Failed to load config: {e}"),
        })?;

        match env_type {
            EnvironmentType::Java => config.clear_default_java_env(),
            EnvironmentType::Cc => config.clear_default_cc_env(),
            _ => {
                return Err(AppError::Validation {
                    field: "env_type".to_string(),
                    reason:
                        "Default environment support is currently only available for Java and CC"
                            .to_string(),
                }
                .into())
            }
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
    ) -> ContextualResult<Option<String>> {
        let config = Config::load().map_err(|e| AppError::Config {
            message: format!("Failed to load config: {e}"),
        })?;

        let default_env = match env_type {
            EnvironmentType::Java => config.default_java_env.clone(),
            EnvironmentType::Cc => config.default_cc_env.clone(),
            _ => None,
        };
        Ok(default_env)
    }

    /// 列出环境时显示默认环境标记
    pub async fn list_environments_with_default(
        &self,
        env_type: EnvironmentType,
        output_format: OutputFormat,
    ) -> ContextualResult<String> {
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
            let environments = manager_guard.list().map_err(|e| AppError::Environment {
                message: format!("Failed to get environment list: {e}"),
            })?;
            let current_env = {
                let session_manager = self.session_manager.lock()?;
                session_manager.get_current_environment(env_type).cloned()
            };
            (environments, current_env)
        };
        let default_env = match env_type {
            EnvironmentType::Java => config.default_java_env.clone(),
            EnvironmentType::Cc => config.default_cc_env.clone(),
            _ => None,
        };

        // 格式化输出
        match output_format {
            OutputFormat::Text => {
                use crate::cli::print::{format_envs, EnvItem};
                let mut items = Vec::new();
                for env in environments {
                    let name = env.name.clone();
                    let description = env.description.clone().unwrap_or_default();
                    let is_current = current_env.as_ref() == Some(&name);
                    let is_default = default_env.as_ref() == Some(&name);

                    // 显示环境信息，对于 CC 环境显示模型
                    let extra = if env_type == EnvironmentType::Cc {
                        if let Some(model) = &env.version {
                            if !model.is_empty() {
                                Some(model.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    items.push(EnvItem {
                        name,
                        description,
                        extra,
                        is_current,
                        is_default,
                    });
                }
                Ok(format_envs(&items))
            }
            OutputFormat::Json => {
                use serde_json;
                let json_output = serde_json::json!({
                    "environment_type": env_type,
                    "current": current_env,
                    "default": default_env,
                    "environments": environments
                });
                Ok(crate::error::safe_to_json_pretty(&json_output)?)
            }
        }
    }
}
