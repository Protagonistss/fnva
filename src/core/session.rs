use crate::core::environment_manager::EnvironmentType;
use crate::infrastructure::config::Config;
use crate::infrastructure::shell::current_envs::CurrentEnvsFile;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;

/// 会话管理器:当前激活环境(与 shell autoload 共用 current_envs.toml 单一存储)。
#[derive(Debug, Clone)]
pub struct SessionManager {
    current_environments: HashMap<EnvironmentType, String>,
}

impl SessionManager {
    /// 创建新的会话管理器,从 current_envs.toml 加载当前环境。
    pub fn new() -> Result<Self, String> {
        let mut manager = Self {
            current_environments: HashMap::new(),
        };
        manager.load_from_current_envs();
        Ok(manager)
    }

    /// 从 current_envs.toml 读取当前环境到内存。
    fn load_from_current_envs(&mut self) {
        if let Ok(file) = CurrentEnvsFile::read() {
            if let Some(v) = file.cc {
                self.current_environments.insert(EnvironmentType::Cc, v);
            }
            if let Some(v) = file.java {
                self.current_environments.insert(EnvironmentType::Java, v);
            }
            if let Some(v) = file.maven {
                self.current_environments.insert(EnvironmentType::Maven, v);
            }
        }
    }

    /// 设置当前环境(写 current_envs.toml)。
    pub fn set_current_environment(
        &mut self,
        env_type: EnvironmentType,
        name: &str,
    ) -> Result<(), String> {
        self.current_environments.insert(env_type, name.to_string());
        CurrentEnvsFile::write(env_type, name)
    }

    /// 获取当前环境
    pub fn get_current_environment(&self, env_type: EnvironmentType) -> Option<&String> {
        self.current_environments.get(&env_type)
    }

    /// 移除当前环境(清 current_envs.toml 对应字段)。
    pub fn remove_current_environment(&mut self, env_type: EnvironmentType) -> Result<(), String> {
        self.current_environments.remove(&env_type);
        CurrentEnvsFile::clear(env_type)
    }

    /// 获取所有当前环境
    pub fn get_all_current(&self) -> &HashMap<EnvironmentType, String> {
        &self.current_environments
    }

    /// 清除所有环境
    pub fn clear_all(&mut self) -> Result<(), String> {
        for env_type in [
            EnvironmentType::Java,
            EnvironmentType::Cc,
            EnvironmentType::Maven,
        ] {
            CurrentEnvsFile::clear(env_type)?;
        }
        self.current_environments.clear();
        Ok(())
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
            EnvironmentType::Maven => config.default_maven_env.as_deref(),
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
            EnvironmentType::Maven => config.default_maven_env.as_deref(),
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
    history: VecDeque<SwitchHistory>,
    /// 最大历史记录数
    max_history: usize,
    /// 历史文件路径
    history_path: PathBuf,
}

impl HistoryManager {
    /// 创建新的历史管理器
    pub fn new(max_history: usize) -> Result<Self, String> {
        let history_path = crate::infrastructure::paths::history_path()?;

        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let mut history_manager = Self {
            history: VecDeque::new(),
            max_history,
            history_path,
        };

        // 加载现有历史
        if let Err(e) = history_manager.load_history() {
            eprintln!("⚠  Failed to load history: {e}");
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

        self.history = parsed_history.into_iter().collect();

        // 限制历史记录数量
        let mut needs_save = false;
        while self.history.len() > self.max_history {
            self.history.pop_front();
            needs_save = true;
        }

        if needs_save {
            let _ = self.save_history();
        }

        Ok(())
    }

    /// 保存历史记录
    fn save_history(&self) -> Result<(), String> {
        #[derive(Serialize)]
        struct HistoryFile<'a> {
            history: &'a VecDeque<SwitchHistory>,
        }
        // 尝试序列化历史记录，如果失败则跳过（为了向后兼容）
        let content = match toml::to_string_pretty(&HistoryFile {
            history: &self.history,
        }) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("⚠  Failed to serialize history: {e}. Skipping history save.");
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

        self.history.push_back(record);

        // 限制历史记录数量
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }

        // 尝试保存历史，但不影响主要功能
        if let Err(e) = self.save_history() {
            eprintln!("⚠  Failed to save history: {e}");
        }

        Ok(())
    }

    /// 获取最近的历史记录
    pub fn get_recent_history(&self, limit: usize) -> Vec<&SwitchHistory> {
        let start = if self.history.len() > limit {
            self.history.len() - limit
        } else {
            0
        };
        self.history.iter().skip(start).collect()
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
