use crate::core::environment_manager::EnvironmentType;
use crate::infrastructure::config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// 会话状态管理器
#[derive(Debug, Clone)]
pub struct SessionManager {
    /// 当前激活的环境
    current_environments: HashMap<EnvironmentType, String>,
    /// 配置文件路径
    config_path: PathBuf,
}

/// 持久化的会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// 当前环境
    pub current_environments: HashMap<EnvironmentType, String>,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            current_environments: HashMap::new(),
            last_updated: chrono::Utc::now(),
        }
    }
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Result<Self, String> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| "Cannot get user home directory".to_string())?
            .join(".fnva");

        // 确保目录存在
        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;

        let config_path = config_dir.join("session.toml");

        let mut session_manager = Self {
            current_environments: HashMap::new(),
            config_path,
        };

        // 加载现有的会话状态
        if let Err(e) = session_manager.load_state() {
            eprintln!("Warning: Failed to load session state: {e}");
        }

        Ok(session_manager)
    }

    /// 加载会话状态
    pub fn load_state(&mut self) -> Result<(), String> {
        if !self.config_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.config_path)
            .map_err(|e| format!("Failed to read session file: {e}"))?;

        let state: SessionState =
            toml::from_str(&content).map_err(|e| format!("Failed to parse session file: {e}"))?;

        self.current_environments = state.current_environments;

        Ok(())
    }

    /// 保存会话状态
    pub fn save_state(&self) -> Result<(), String> {
        let state = SessionState {
            current_environments: self.current_environments.clone(),
            last_updated: chrono::Utc::now(),
        };

        let content = toml::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize session state: {e}"))?;

        fs::write(&self.config_path, content)
            .map_err(|e| format!("Failed to write session file: {e}"))?;

        Ok(())
    }

    /// 设置当前环境
    pub fn set_current_environment(
        &mut self,
        env_type: EnvironmentType,
        name: &str,
    ) -> Result<(), String> {
        self.current_environments.insert(env_type, name.to_string());
        self.save_state()
    }

    /// 获取当前环境
    pub fn get_current_environment(&self, env_type: EnvironmentType) -> Option<&String> {
        self.current_environments.get(&env_type)
    }

    /// 移除当前环境
    pub fn remove_current_environment(&mut self, env_type: EnvironmentType) -> Result<(), String> {
        self.current_environments.remove(&env_type);
        self.save_state()
    }

    /// 获取所有当前环境
    pub fn get_all_current(&self) -> &HashMap<EnvironmentType, String> {
        &self.current_environments
    }

    /// 清除所有环境
    pub fn clear_all(&mut self) -> Result<(), String> {
        self.current_environments.clear();
        self.save_state()
    }

    /// 检查环境是否为当前激活
    pub fn is_current_environment(&self, env_type: EnvironmentType, name: &str) -> bool {
        self.current_environments
            .get(&env_type)
            .map(|current| current == name)
            .unwrap_or(false)
    }

    /// 从配置中同步当前环境（用于迁移）
    pub fn sync_from_config(&mut self, config: &Config) -> Result<(), String> {
        // 同步 Java 环境
        if let Some(ref java_env) = config.current_java_env {
            self.set_current_environment(EnvironmentType::Java, java_env)?;
        }

        // TODO: 同步其他环境类型

        Ok(())
    }

    /// 导出到配置（用于保存到主配置文件）
    pub fn export_to_config(&self, config: &mut Config) -> Result<(), String> {
        if let Some(java_env) = self.get_current_environment(EnvironmentType::Java) {
            config.current_java_env = Some(java_env.clone());
        }

        // TODO: 导出其他环境类型

        Ok(())
    }

    /// 获取有效的环境（当前环境或默认环境）
    pub fn get_effective_environment<'a>(
        &'a self,
        config: &'a Config,
        env_type: EnvironmentType,
    ) -> Option<&'a str> {
        // 首先尝试获取当前环境
        if let Some(current) = self.get_current_environment(env_type) {
            return Some(current);
        }

        // 如果没有当前环境，尝试获取默认环境（仅支持 Java）
        match env_type {
            EnvironmentType::Java => config.default_java_env.as_deref(),
            EnvironmentType::Cc => config.default_cc_env.as_deref(),
            _ => None,
        }
    }

    /// 检查环境是否是当前环境或默认环境
    pub fn is_active_environment(
        &self,
        config: &Config,
        env_type: EnvironmentType,
        name: &str,
    ) -> bool {
        // 检查是否是当前环境
        if let Some(current) = self.get_current_environment(env_type) {
            if current == name {
                return true;
            }
        }

        // 检查是否是默认环境（仅支持 Java）
        if let Some(default) = match env_type {
            EnvironmentType::Java => config.default_java_env.as_deref(),
            EnvironmentType::Cc => config.default_cc_env.as_deref(),
            _ => None,
        } {
            if default == name {
                return true;
            }
        }

        false
    }
}

/// 环境切换历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchHistory {
    /// 环境类型
    pub env_type: EnvironmentType,
    /// 旧环境名称
    pub old_env: Option<String>,
    /// 新环境名称
    pub new_env: String,
    /// 切换时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// 切换原因
    pub reason: Option<String>,
}

/// 环境历史管理器
#[derive(Debug)]
pub struct HistoryManager {
    /// 切换历史
    history: Vec<SwitchHistory>,
    /// 最大历史记录数
    max_history: usize,
    /// 历史文件路径
    history_path: PathBuf,
}

impl HistoryManager {
    /// 创建新的历史管理器
    pub fn new(max_history: usize) -> Result<Self, String> {
        let config_dir = dirs::home_dir()
            .ok_or_else(|| "Cannot get user home directory".to_string())?
            .join(".fnva");

        fs::create_dir_all(&config_dir)
            .map_err(|e| format!("Failed to create config directory: {e}"))?;

        let history_path = config_dir.join("history.toml");

        let mut history_manager = Self {
            history: Vec::new(),
            max_history,
            history_path,
        };

        // 加载现有历史
        if let Err(e) = history_manager.load_history() {
            eprintln!("Warning: Failed to load history: {e}");
        }

        Ok(history_manager)
    }

    /// 加载历史记录
    fn load_history(&mut self) -> Result<(), String> {
        if !self.history_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.history_path)
            .map_err(|e| format!("Failed to read history file: {e}"))?;

        #[derive(Deserialize)]
        struct HistoryFile {
            history: Vec<SwitchHistory>,
        }

        let parsed_history = toml::from_str::<HistoryFile>(&content)
            .map(|file| file.history)
            .or_else(|_| toml::from_str::<Vec<SwitchHistory>>(&content))
            .map_err(|e| format!("Failed to parse history file: {e}"))?;

        self.history = parsed_history;

        // 限制历史记录数量
        if self.history.len() > self.max_history {
            self.history.truncate(self.max_history);
        }

        Ok(())
    }

    /// 保存历史记录
    fn save_history(&self) -> Result<(), String> {
        #[derive(Serialize)]
        struct HistoryFile<'a> {
            history: &'a [SwitchHistory],
        }
        // 尝试序列化历史记录，如果失败则跳过（为了向后兼容）
        let content = match toml::to_string_pretty(&HistoryFile {
            history: &self.history,
        }) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Warning: Failed to serialize history: {e}. Skipping history save.");
                return Ok(());
            }
        };

        fs::write(&self.history_path, content)
            .map_err(|e| format!("Failed to write history file: {e}"))?;

        Ok(())
    }

    /// 记录环境切换
    pub fn record_switch(
        &mut self,
        env_type: EnvironmentType,
        old_env: Option<String>,
        new_env: String,
        reason: Option<String>,
    ) -> Result<(), String> {
        let record = SwitchHistory {
            env_type,
            old_env,
            new_env,
            timestamp: chrono::Utc::now(),
            reason,
        };

        self.history.push(record);

        // 限制历史记录数量
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        // 尝试保存历史，但不影响主要功能
        if let Err(e) = self.save_history() {
            eprintln!("Warning: Failed to save history: {e}");
        }

        Ok(())
    }

    /// 获取最近的历史记录
    pub fn get_recent_history(&self, limit: usize) -> &[SwitchHistory] {
        let start = if self.history.len() > limit {
            self.history.len() - limit
        } else {
            0
        };
        &self.history[start..]
    }

    /// 获取特定环境类型的历史
    pub fn get_history_for_env(&self, env_type: EnvironmentType) -> Vec<&SwitchHistory> {
        self.history
            .iter()
            .filter(|record| record.env_type == env_type)
            .collect()
    }

    /// 清除历史记录
    pub fn clear_history(&mut self) -> Result<(), String> {
        self.history.clear();
        self.save_history()
    }
}
