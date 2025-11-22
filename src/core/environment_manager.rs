use crate::infrastructure::shell::ShellType;
use serde::{Deserialize, Serialize};

/// 环境类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EnvironmentType {
    Java,
    Llm,
    Cc,
    Maven,
    Gradle,
    Python,
    Node,
}

impl std::fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvironmentType::Java => write!(f, "java"),
            EnvironmentType::Llm => write!(f, "llm"),
            EnvironmentType::Cc => write!(f, "cc"),
            EnvironmentType::Maven => write!(f, "maven"),
            EnvironmentType::Gradle => write!(f, "gradle"),
            EnvironmentType::Python => write!(f, "python"),
            EnvironmentType::Node => write!(f, "node"),
        }
    }
}

/// 环境信息动态类型
#[derive(Debug, Clone, Serialize)]
pub struct DynEnvironment {
    pub name: String,
    pub path: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
}

/// 环境管理器抽象接口（对象安全版本）
pub trait EnvironmentManager: Send + Sync {
    /// 获取环境类型
    fn environment_type(&self) -> EnvironmentType;

    /// 列出所有环境
    fn list(&self) -> Result<Vec<DynEnvironment>, String>;

    /// 根据名称获取环境
    fn get(&self, name: &str) -> Result<Option<DynEnvironment>, String>;

    /// 添加环境（使用字符串配置以保持对象安全）
    fn add(&mut self, name: &str, config_str: &str) -> Result<(), String>;

    /// 删除环境
    fn remove(&mut self, name: &str) -> Result<(), String>;

    /// 使用环境（生成 shell 脚本）
    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, String>;

    /// 获取当前环境名称
    fn get_current(&self) -> Result<Option<String>, String>;

    /// 设置当前环境
    fn set_current(&mut self, name: &str) -> Result<(), String>;

    /// 扫描系统中的可用环境
    fn scan(&self) -> Result<Vec<DynEnvironment>, String>;

    /// 检查环境是否可用
    fn is_available(&self, name: &str) -> Result<bool, String>;

    /// 获取环境的详细信息
    fn get_details(&self, name: &str) -> Result<Option<DynEnvironment>, String>;
}

/// 环境配置的通用接口
pub trait EnvironmentConfig {
    /// 获取环境名称
    fn name(&self) -> &str;

    /// 获取环境描述
    fn description(&self) -> &str;

    /// 验证配置是否有效
    fn validate(&self) -> Result<(), String>;
}

/// 环境信息的通用接口
pub trait EnvironmentInfo {
    /// 获取环境名称
    fn name(&self) -> &str;

    /// 获取环境描述
    fn description(&self) -> &str;

    /// 获取环境是否激活
    fn is_active(&self) -> bool;

    /// 获取环境路径或主要标识
    fn get_identifier(&self) -> &str;
}

/// 环境切换结果
#[derive(Debug, Clone, Serialize)]
pub struct SwitchResult {
    /// 环境名称
    pub name: String,
    /// 环境类型
    pub env_type: EnvironmentType,
    /// 生成的 shell 脚本
    pub script: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 环境管理器的统一工厂
pub struct EnvironmentManagerFactory;

impl EnvironmentManagerFactory {
    /// 根据类型创建对应的环境管理器
    pub fn create_manager(
        env_type: EnvironmentType,
    ) -> Result<Box<dyn EnvironmentManager>, String> {
        match env_type {
            EnvironmentType::Java => Ok(Box::new(
                crate::environments::java::JavaEnvironmentManager::new(),
            )),
            EnvironmentType::Llm => Ok(Box::new(
                crate::environments::llm::LlmEnvironmentManager::new(),
            )),
            EnvironmentType::Cc => Ok(Box::new(
                crate::environments::cc::CcEnvironmentManager::new(),
            )),
            EnvironmentType::Maven => {
                Err("Maven environment manager not implemented yet".to_string())
            }
            EnvironmentType::Gradle => {
                Err("Gradle environment manager not implemented yet".to_string())
            }
            EnvironmentType::Python => {
                Err("Python environment manager not implemented yet".to_string())
            }
            EnvironmentType::Node => {
                Err("Node environment manager not implemented yet".to_string())
            }
        }
    }
}
