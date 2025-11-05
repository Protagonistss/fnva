use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::core::environment_manager::{EnvironmentManager, EnvironmentType, SwitchResult};
use crate::core::session::{SessionManager, HistoryManager};
use crate::infrastructure::shell::{ShellType, script_builder::ScriptBuilder};
use crate::cli::output::OutputFormat;

/// 环境切换器
pub struct EnvironmentSwitcher {
    /// 环境管理器
    managers: HashMap<EnvironmentType, Arc<Mutex<dyn EnvironmentManager>>>,
    /// 会话管理器
    session_manager: Arc<Mutex<SessionManager>>,
    /// 历史管理器
    history_manager: Arc<Mutex<HistoryManager>>,
}

impl EnvironmentSwitcher {
    /// 创建新的环境切换器
    pub fn new() -> Result<Self, String> {
        let session_manager = Arc::new(Mutex::new(SessionManager::new()?));
        let history_manager = Arc::new(Mutex::new(HistoryManager::new(100)?));

        Ok(Self {
            managers: HashMap::new(),
            session_manager,
            history_manager,
        })
    }

    /// 注册环境管理器
    pub fn register_manager(&mut self, manager: Arc<Mutex<dyn EnvironmentManager>>) {
        let env_type = manager.lock().unwrap().environment_type();
        self.managers.insert(env_type, manager);
    }

    /// 切换环境
    pub async fn switch_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
        shell_type: Option<ShellType>,
        reason: Option<String>,
    ) -> Result<SwitchResult, String> {
        // 获取环境管理器
        let manager = self.managers.get(&env_type)
            .ok_or_else(|| format!("No manager registered for environment type: {:?}", env_type))?;

        // 获取当前环境（不可变）
        let old_env = {
            let manager = manager.lock().unwrap();
            manager.get_current()?
        };

        // 如果已经是当前环境，直接返回
        if let Some(ref current) = old_env {
            if current == name {
                return Ok(SwitchResult {
                    name: name.to_string(),
                    env_type,
                    script: String::new(),
                    success: true,
                    error: None,
                });
            }
        }

        // 验证环境是否存在
        let env_info = {
            let manager = manager.lock().unwrap();
            manager.get(name)?
        };
        if env_info.is_none() {
            return Ok(SwitchResult {
                name: name.to_string(),
                env_type,
                script: String::new(),
                success: false,
                error: Some(format!("Environment '{}' not found", name)),
            });
        }

        // 生成切换脚本（需要可变借用）
        let script = {
            let mut manager = manager.lock().unwrap();
            manager.use_env(name, shell_type)?
        };

        // 更新会话状态
        {
            let mut session_manager = self.session_manager.lock().unwrap();
            session_manager.set_current_environment(env_type, name)?;
        }

        // 记录历史
        {
            let mut history_manager = self.history_manager.lock().unwrap();
            history_manager.record_switch(env_type, old_env, name.to_string(), reason)?;
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
    ) -> Result<String, String> {
        let manager = self.managers.get(&env_type)
            .ok_or_else(|| format!("No manager registered for environment type: {:?}", env_type))?;

        let manager = manager.lock().unwrap();
        let environments = manager.list()?;

        // 获取当前环境
        let current_env = {
            let session_manager = self.session_manager.lock().unwrap();
            session_manager.get_current_environment(env_type).cloned()
        };

        // 格式化输出
        match output_format {
            OutputFormat::Text => {
                let mut output = String::new();
                if environments.is_empty() {
                    output.push_str(&format!("No {} environments found\n", env_type));
                } else {
                    output.push_str(&format!("Available {} environments:\n", env_type));
                    for env in environments {
                        let name = env.name.clone();
                        let description = env.description.clone().unwrap_or_default();
                        let is_current = current_env.as_ref().map_or(false, |curr| curr == &name);
                        let marker = if is_current { " (current)" } else { "" };
                        output.push_str(&format!("  {}{}: {}\n", name, marker, description));
                    }
                }
                Ok(output)
            }
            OutputFormat::Json => {
                use serde_json;
                let json_output = serde_json::json!({
                    "environment_type": env_type,
                    "current": current_env,
                    "environments": environments
                });
                Ok(serde_json::to_string_pretty(&json_output).unwrap())
            }
        }
    }

    /// 添加环境
    pub async fn add_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
        config: serde_json::Value,
    ) -> Result<String, String> {
        let manager = self.managers.get(&env_type)
            .ok_or_else(|| format!("No manager registered for environment type: {:?}", env_type))?;

        // 这里需要根据环境类型解析配置
        // TODO: 实现配置解析逻辑

        let mut manager = manager.lock().unwrap();

        // Convert JSON Value to string for the object-safe interface
        let config_str = config.to_string();
        manager.add(name, &config_str)?;

        Ok(format!("Successfully added {} environment: {}", env_type, name))
    }

    /// 删除环境
    pub async fn remove_environment(
        &self,
        env_type: EnvironmentType,
        name: &str,
    ) -> Result<String, String> {
        let manager = self.managers.get(&env_type)
            .ok_or_else(|| format!("No manager registered for environment type: {:?}", env_type))?;

        let mut manager = manager.lock().unwrap();
        manager.remove(name)?;

        // 如果删除的是当前环境，清除会话状态
        {
            let mut session_manager = self.session_manager.lock().unwrap();
            if let Some(current) = session_manager.get_current_environment(env_type) {
                if current == name {
                    session_manager.remove_current_environment(env_type)?;
                }
            }
        }

        Ok(format!("Successfully removed {} environment: {}", env_type, name))
    }

    /// 获取当前环境
    pub async fn get_current_environment(
        &self,
        env_type: EnvironmentType,
        output_format: OutputFormat,
    ) -> Result<String, String> {
        let manager = self.managers.get(&env_type)
            .ok_or_else(|| format!("No manager registered for environment type: {:?}", env_type))?;

        let manager = manager.lock().unwrap();
        let current_env = manager.get_current()?;

        match output_format {
            OutputFormat::Text => {
                if let Some(env_name) = current_env {
                    if let Some(env_info) = manager.get(&env_name)? {
                        Ok(format!("Current {} environment: {}\n{}\n",
                            env_type, env_name, env_info.description.clone().unwrap_or_default()))
                    } else {
                        Ok(format!("Current {} environment: {} (details unavailable)\n", env_type, env_name))
                    }
                } else {
                    Ok(format!("No current {} environment\n", env_type))
                }
            }
            OutputFormat::Json => {
                use serde_json;
                let json_output = if let Some(env_name) = current_env {
                    if let Some(env_info) = manager.get(&env_name)? {
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
                Ok(serde_json::to_string_pretty(&json_output).unwrap())
            }
        }
    }

    /// 生成 shell 集成脚本
    pub async fn generate_shell_integration(
        &self,
        shell_type: ShellType,
    ) -> Result<String, String> {
        let session_manager = self.session_manager.lock().unwrap();
        let current_envs = session_manager.get_all_current();

        ScriptBuilder::build_integration_script(current_envs, shell_type)
    }

    /// 扫描环境
    pub async fn scan_environments(
        &self,
        env_type: EnvironmentType,
    ) -> Result<String, String> {
        let manager = self.managers.get(&env_type)
            .ok_or_else(|| format!("No manager registered for environment type: {:?}", env_type))?;

        let manager = manager.lock().unwrap();
        let found_envs = manager.scan()?;

        let mut output = String::new();
        if found_envs.is_empty() {
            output.push_str(&format!("No {} environments found on system\n", env_type));
        } else {
            output.push_str(&format!("Found {} {} environments:\n", found_envs.len(), env_type));
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
    ) -> Result<String, String> {
        let history_manager = self.history_manager.lock().unwrap();

        let history: Vec<_> = if let Some(env_type) = env_type {
            history_manager.get_history_for_env(env_type)
                .into_iter()
                .rev()
                .take(limit)
                .collect()
        } else {
            history_manager.get_recent_history(limit)
                .iter()
                .rev()
                .collect()
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

    }