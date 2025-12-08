use crate::cli::output::OutputFormat;
use crate::core::environment_manager::{EnvironmentManager, EnvironmentType, SwitchResult};
use crate::core::session::{HistoryManager, SessionManager, SwitchHistory};
use crate::error::{
    option_with_context, safe_to_json, safe_to_json_pretty, AppError, ContextualResult, SafeMutex,
};
use crate::infrastructure::config::Config;
use crate::infrastructure::shell::{script_factory::ScriptGenerator, ShellType};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
            message: format!("创建会话管理器失败: {e}"),
        })?;
        let history_manager = HistoryManager::new(100).map_err(|e| AppError::Internal {
            message: format!("创建历史管理器失败: {e}"),
        })?;

        Ok(Self {
            managers: HashMap::new(),
            session_manager: SafeMutex::new(session_manager, "session_manager"),
            history_manager: SafeMutex::new(history_manager, "history_manager"),
        })
    }

    /// 注册环境管理器
    pub fn register_manager(
        &mut self,
        manager: Arc<Mutex<dyn EnvironmentManager>>,
    ) -> ContextualResult<()> {
        let env_type = manager.lock().map(|guard| guard.environment_type())?;
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
            "切换环境时查找环境管理器",
        )?;

        // 获取当前环境（不可变）
        let old_env = {
            let manager_guard = manager.lock()?;
            manager_guard
                .get_current()
                .map_err(|e| AppError::Environment {
                    message: format!("获取当前环境失败: {e}"),
                })?
        };

        // 验证环境是否存在
        let env_info = {
            let manager_guard = manager.lock()?;
            manager_guard.get(name).map_err(|e| AppError::Environment {
                message: format!("查找环境 '{name}' 失败: {e}"),
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
            let mut manager_guard = manager.lock()?;
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
                    message: format!("更新会话状态失败: {e}"),
                })?;
        }

        // 记录历史
        {
            let mut history_manager = self.history_manager.lock()?;
            history_manager
                .record_switch(env_type, old_env, name.to_string(), reason)
                .map_err(|e| AppError::Internal {
                    message: format!("记录切换历史失败: {e}"),
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

    /// 列出环境
    pub async fn list_environments(
        &self,
        env_type: EnvironmentType,
        output_format: OutputFormat,
    ) -> ContextualResult<String> {
        let manager = option_with_context(
            self.managers.get(&env_type),
            AppError::env_not_found(&format!("{env_type:?}")),
            "列出环境时查找环境管理器",
        )?;

        let environments = {
            let manager_guard = manager.lock()?;
            manager_guard.list().map_err(|e| AppError::Environment {
                message: format!("获取环境列表失败: {e}"),
            })?
        };

        // 获取当前环境
        let current_env = {
            let session_manager = self.session_manager.lock()?;
            session_manager.get_current_environment(env_type).cloned()
        };

        // 格式化输出
        match output_format {
            OutputFormat::Text => {
                let mut output = String::new();
                if environments.is_empty() {
                    output.push_str(&format!("No {env_type} environments found\n"));
                } else {
                    output.push_str(&format!("Available {env_type} environments:\n"));
                    for env in environments {
                        let name = env.name.clone();
                        let description = env.description.clone().unwrap_or_default();
                        let is_current = current_env.as_ref() == Some(&name);
                        let marker = if is_current { " (current)" } else { "" };
                        output.push_str(&format!("  {name}{marker}: {description}\n"));
                    }
                }
                Ok(output)
            }
            OutputFormat::Json => {
                let json_output = serde_json::json!({
                    "environment_type": env_type,
                    "current": current_env,
                    "environments": environments
                });
                Ok(safe_to_json_pretty(&json_output)?)
            }
        }
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
            "添加环境时查找环境管理器",
        )?;

        // 这里需要根据环境类型解析配置
        // TODO: 实现配置解析逻辑

        let result = {
            let mut manager_guard = manager.lock()?;

            // Convert JSON Value to string for the object-safe interface
            let config_str = safe_to_json(&config)?;

            manager_guard
                .add(name, &config_str)
                .map_err(|e| AppError::Environment {
                    message: format!("添加环境失败: {e}"),
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
            "删除环境时查找环境管理器",
        )?;

        {
            let mut manager_guard = manager.lock()?;
            manager_guard
                .remove(name)
                .map_err(|e| AppError::Environment {
                    message: format!("删除环境失败: {e}"),
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
                            message: format!("清除当前环境失败: {e}"),
                        })?;
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
            "获取当前环境时查找环境管理器",
        )?;

        let (current_env, manager_guard) = {
            let manager_guard = manager.lock()?;
            let current_env = manager_guard
                .get_current()
                .map_err(|e| AppError::Environment {
                    message: format!("获取当前环境失败: {e}"),
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
                                message: format!("获取环境信息失败: {e}"),
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
                                message: format!("获取环境信息失败: {e}"),
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
            "扫描环境时查找环境管理器",
        )?;

        let found_envs = {
            let manager_guard = manager.lock()?;
            manager_guard.scan().map_err(|e| AppError::Environment {
                message: format!("扫描环境失败: {e}"),
            })?
        };

        let mut output = String::new();
        if found_envs.is_empty() {
            output.push_str(&format!("No {env_type} environments found on system\n"));
        } else {
            output.push_str(&format!(
                "Found {} {} environments:\n",
                found_envs.len(),
                env_type
            ));
            for env in found_envs {
                output.push_str(&format!("  {}: {}\n", env.name, env.path));
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
                // get_recent_history returns &[SwitchHistory]
                history_manager
                    .get_recent_history(limit)
                    .iter()
                    .rev()
                    .cloned()
                    .collect()
            }
        };

        let mut output = String::new();
        if history.is_empty() {
            output.push_str("No switch history found\n");
        } else {
            output.push_str("Recent environment switches:\n");
            for record in history {
                output.push_str(&format!(
                    "{} {} -> {} ({})\n",
                    record.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    record.old_env.as_deref().unwrap_or("None"),
                    record.new_env,
                    record.env_type
                ));
            }
        }

        Ok(output)
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
            "设置默认环境时查找环境管理器",
        )?;

        {
            let manager = manager_entry.lock()?;
            if !manager
                .is_available(name)
                .map_err(|e| AppError::Environment {
                    message: format!("检查环境可用性失败: {e}"),
                })?
            {
                return Err(AppError::Environment {
                    message: format!("{env_type} environment '{name}' not found"),
                }
                .into());
            }
        }

        // 直接设置默认环境（不验证）
        let mut config = Config::load().map_err(|e| AppError::Config {
            message: format!("加载配置失败: {e}"),
        })?;

        match env_type {
            EnvironmentType::Java => {
                config
                    .set_default_java_env(name.to_string())
                    .map_err(|e| AppError::Config {
                        message: format!("设置默认Java环境失败: {e}"),
                    })?
            }
            EnvironmentType::Cc => {
                config
                    .set_default_cc_env(name.to_string())
                    .map_err(|e| AppError::Config {
                        message: format!("设置默认CC环境失败: {e}"),
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
            message: format!("保存配置失败: {e}"),
        })?;

        Ok(format!("Set default {env_type} environment: {name}"))
    }

    /// 清除默认环境
    pub async fn clear_default_environment(
        &self,
        env_type: EnvironmentType,
    ) -> ContextualResult<String> {
        let mut config = Config::load().map_err(|e| AppError::Config {
            message: format!("加载配置失败: {e}"),
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
            message: format!("保存配置失败: {e}"),
        })?;

        Ok(format!("Cleared default {env_type} environment"))
    }

    /// 获取默认环境
    pub async fn get_default_environment(
        &self,
        env_type: EnvironmentType,
    ) -> ContextualResult<Option<String>> {
        let config = Config::load().map_err(|e| AppError::Config {
            message: format!("加载配置失败: {e}"),
        })?;

        let default_env = match env_type {
            EnvironmentType::Java => config.default_java_env.clone(),
            EnvironmentType::Cc => config.default_cc_env.clone(),
            _ => None,
        };
        Ok(default_env)
    }

    /// 切换到默认环境
    pub async fn switch_to_default_environment(
        &self,
        env_type: EnvironmentType,
        shell_type: Option<ShellType>,
    ) -> ContextualResult<SwitchResult> {
        let config = Config::load().map_err(|e| AppError::Config {
            message: format!("加载配置失败: {e}"),
        })?;

        let default_env = match env_type {
            EnvironmentType::Java => config.default_java_env.clone(),
            EnvironmentType::Cc => config.default_cc_env.clone(),
            _ => None,
        };

        if let Some(default_env) = default_env {
            self.switch_environment(
                env_type,
                &default_env,
                shell_type,
                Some("Switch to default environment".to_string()),
            )
            .await
        } else {
            Ok(SwitchResult {
                name: "default".to_string(),
                env_type,
                script: String::new(),
                success: false,
                error: Some(format!("No default {env_type} environment set")),
            })
        }
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
            "列出环境时查找环境管理器",
        )?;

        // 一次性加载配置，避免重复读取
        let config = Config::load().map_err(|e| AppError::Config {
            message: format!("加载配置失败: {e}"),
        })?;

        let (environments, current_env) = {
            let manager_guard = manager.lock()?;
            let environments = manager_guard.list().map_err(|e| AppError::Environment {
                message: format!("获取环境列表失败: {e}"),
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
                let mut output = String::new();
                if environments.is_empty() {
                    output.push_str(&format!("No {env_type} environments found\n"));
                } else {
                    output.push_str(&format!("Available {env_type} environments:\n"));
                    for env in environments {
                        let name = env.name.clone();
                        let description = env.description.clone().unwrap_or_default();
                        let is_current = current_env.as_ref() == Some(&name);
                        let is_default = default_env.as_ref() == Some(&name);

                        let mut markers = Vec::new();
                        if is_current {
                            markers.push("current");
                        }
                        if is_default {
                            markers.push("default");
                        }
                        let marker_str = if markers.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", markers.join(", "))
                        };

                        // 显示环境信息，对于 CC 环境显示模型
                        let env_info = if env_type == EnvironmentType::Cc {
                            if let Some(model) = &env.version {
                                if !model.is_empty() {
                                    format!(" - {model}")
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };

                        output
                            .push_str(&format!("  {name}{marker_str}: {description}{env_info}\n"));
                    }
                }
                Ok(output)
            }
            OutputFormat::Json => {
                use serde_json;
                let json_output = serde_json::json!({
                    "environment_type": env_type,
                    "current": current_env,
                    "default": default_env,
                    "environments": environments
                });
                Ok(safe_to_json_pretty(&json_output)?)
            }
        }
    }
}
