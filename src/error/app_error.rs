use std::io;
use thiserror::Error;

/// 应用程序错误类型
#[derive(Error, Debug, Clone)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),

    #[error("环境管理错误: {message}")]
    Environment { message: String },

    #[error("配置错误: {message}")]
    Config { message: String },

    #[error("网络错误: {message}")]
    Network { message: String },

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("路径错误: {path} - {reason}")]
    Path { path: String, reason: String },

    #[error("线程锁定错误: {operation}")]
    LockError { operation: String },

    #[error("版本解析错误: {version}")]
    VersionParse { version: String },

    #[error("安装错误: {message}")]
    Installation { message: String },

    #[error("Shell 脚本生成错误: {shell_type} - {reason}")]
    ScriptGeneration { shell_type: String, reason: String },

    #[error("未找到请求的资源: {resource}")]
    NotFound { resource: String },

    #[error("权限错误: {operation}")]
    Permission { operation: String },

    #[error("验证错误: {field} - {reason}")]
    Validation { field: String, reason: String },

    #[error("内部错误: {message}")]
    Internal { message: String },
}

/// 用于提供错误上下文和用户友好建议
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub suggestions: Vec<String>,
    pub help_url: Option<String>,
}

impl AppError {
    /// 为错误添加上下文信息
    pub fn with_context(self, operation: &str) -> ContextualError {
        ContextualError {
            error: self,
            context: ErrorContext {
                operation: operation.to_string(),
                suggestions: Vec::new(),
                help_url: None,
            },
        }
    }

    /// 为错误添加建议
    pub fn with_suggestions(mut self, suggestions: Vec<&str>) -> Self {
        if let AppError::Environment { message } = &mut self {
            *message = format!("{}\n建议: {}", message, suggestions.join(", "));
        }
        self
    }
}

/// 带有上下文的错误
#[derive(Error, Debug)]
pub struct ContextualError {
    #[source]
    pub error: AppError,
    pub context: ErrorContext,
}

impl std::fmt::Display for ContextualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "操作失败: {}\n错误: {}", self.context.operation, self.error)
    }
}

impl ContextualError {
    /// 获取用户友好的错误消息
    pub fn user_message(&self) -> String {
        let mut msg = format!("❌ {}\n", self.context.operation);
        msg.push_str(&format!("原因: {}\n", self.error));

        if !self.context.suggestions.is_empty() {
            msg.push_str("💡 建议:\n");
            for suggestion in &self.context.suggestions {
                msg.push_str(&format!("  • {}\n", suggestion));
            }
        }

        if let Some(help_url) = &self.context.help_url {
            msg.push_str(&format!("📖 更多帮助: {}\n", help_url));
        }

        msg
    }
}

/// 应用程序 Result 类型
pub type AppResult<T> = Result<T, AppError>;
pub type ContextualResult<T> = Result<T, ContextualError>;

/// 便捷的错误创建函数
impl AppError {
    pub fn env_not_found(name: &str) -> Self {
        Self::Environment {
            message: format!("未找到环境: {}", name),
        }
    }

    pub fn config_load_failed(path: &str, reason: &str) -> Self {
        Self::Config {
            message: format!("无法加载配置文件 {}: {}", path, reason),
        }
    }

    pub fn lock_failed(operation: &str) -> Self {
        Self::LockError {
            operation: operation.to_string(),
        }
    }

    pub fn path_conversion_failed(path: &str) -> Self {
        Self::Path {
            path: path.to_string(),
            reason: "路径包含无效字符".to_string(),
        }
    }

    pub fn version_parse_failed(version: &str) -> Self {
        Self::VersionParse {
            version: version.to_string(),
        }
    }

    pub fn permission_denied(operation: &str) -> Self {
        Self::Permission {
            operation: operation.to_string(),
        }
    }
}

// 必要的trait实现
impl From<AppError> for ContextualError {
    fn from(error: AppError) -> Self {
        Self {
            error,
            context: ErrorContext {
                operation: "未知操作".to_string(),
                suggestions: Vec::new(),
                help_url: None,
            },
        }
    }
}

impl From<ContextualError> for String {
    fn from(error: ContextualError) -> Self {
        error.user_message()
    }
}