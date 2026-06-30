use thiserror::Error;

/// 应用程序错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO 错误: {0}")]
    Io(String),

    #[error("环境管理错误: {message}")]
    Environment { message: String },

    #[error("配置错误: {message}")]
    Config { message: String },

    #[error("网络错误: {message}")]
    Network { message: String },

    #[error("序列化错误: {0}")]
    Serialization(String),

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
        write!(
            f,
            "操作失败: {}\n错误: {}",
            self.context.operation, self.error
        )
    }
}

impl ContextualError {
    /// 获取用户友好的错误消息
    pub fn user_message(&self) -> String {
        let mut msg = format!("{}\n", self.context.operation);
        msg.push_str(&format!("Cause: {}\n", self.error));

        if !self.context.suggestions.is_empty() {
            msg.push_str("Suggestions:\n");
            for suggestion in &self.context.suggestions {
                msg.push_str(&format!("  - {suggestion}\n"));
            }
        }

        if let Some(help_url) = &self.context.help_url {
            msg.push_str(&format!("Help: {help_url}\n"));
        }

        msg
    }
}

/// 应用程序 Result 类型
pub type AppResult<T> = Result<T, AppError>;
pub type ContextualResult<T> = Result<T, Box<ContextualError>>;

/// 便捷的错误创建函数
impl AppError {
    pub fn env_not_found(name: &str) -> Self {
        Self::Environment {
            message: format!("未找到环境: {name}"),
        }
    }

    pub fn config_load_failed(path: &str, reason: &str) -> Self {
        Self::Config {
            message: format!("无法加载配置文件 {path}: {reason}"),
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

// 转换trait实现
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        AppError::Io(error.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        AppError::Serialization(error.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(error: toml::de::Error) -> Self {
        AppError::Serialization(error.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(error: toml::ser::Error) -> Self {
        AppError::Serialization(error.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for AppError {
    fn from(_error: std::sync::PoisonError<T>) -> Self {
        AppError::LockError {
            operation: "线程锁定失败".to_string(),
        }
    }
}

impl From<handlebars::TemplateError> for AppError {
    fn from(error: handlebars::TemplateError) -> Self {
        AppError::Serialization(error.to_string())
    }
}

impl From<handlebars::RenderError> for AppError {
    fn from(error: handlebars::RenderError) -> Self {
        AppError::Serialization(error.to_string())
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

impl<T> From<std::sync::PoisonError<T>> for ContextualError {
    fn from(_error: std::sync::PoisonError<T>) -> Self {
        Self {
            error: AppError::LockError {
                operation: "线程锁定失败".to_string(),
            },
            context: ErrorContext {
                operation: "锁定失败".to_string(),
                suggestions: vec!["检查是否存在死锁".to_string()],
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

impl From<Box<ContextualError>> for String {
    fn from(error: Box<ContextualError>) -> Self {
        error.user_message()
    }
}

/// 为 Result 添加上下文信息的扩展 trait
pub trait ResultExt<T> {
    fn with_context(self, operation: &str) -> Result<T, Box<ContextualError>>;
}

impl<T, E: Into<AppError>> ResultExt<T> for Result<T, E> {
    fn with_context(self, operation: &str) -> Result<T, Box<ContextualError>> {
        self.map_err(|e| {
            Box::new(ContextualError {
                error: e.into(),
                context: ErrorContext {
                    operation: operation.to_string(),
                    suggestions: Vec::new(),
                    help_url: None,
                },
            })
        })
    }
}

/// 为所有 Result 类型提供 with_context 方法的便利函数
pub fn with_context<T, E: Into<AppError>>(
    result: Result<T, E>,
    operation: &str,
) -> Result<T, Box<ContextualError>> {
    result.with_context(operation)
}

impl From<AppError> for Box<ContextualError> {
    fn from(error: AppError) -> Self {
        Box::new(ContextualError::from(error))
    }
}

impl<T> From<std::sync::PoisonError<T>> for Box<ContextualError> {
    fn from(error: std::sync::PoisonError<T>) -> Self {
        Box::new(ContextualError::from(error))
    }
}
