use thiserror::Error;

/// 应用程序错误类型
#[derive(Error, Debug)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("Environment management error: {message}")]
    Environment { message: String },

    #[error("Config error: {message}")]
    Config { message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Path error: {path} - {reason}")]
    Path { path: String, reason: String },

    #[error("Thread lock error: {operation}")]
    LockError { operation: String },

    #[error("Version parse error: {version}")]
    VersionParse { version: String },

    #[error("Installation error: {message}")]
    Installation { message: String },

    #[error("Shell script generation error: {shell_type} - {reason}")]
    ScriptGeneration { shell_type: String, reason: String },

    #[error("Requested resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Permission error: {operation}")]
    Permission { operation: String },

    #[error("Validation error: {field} - {reason}")]
    Validation { field: String, reason: String },

    #[error("Internal error: {message}")]
    Internal { message: String },

    /// 带操作上下文的包装错误:把高层操作名(如 "switching to java environment 'x'")
    /// 附在底层错误前,方便定位。用 [`AppError::root_cause`] 可递归剥到具体变体。
    #[error("{operation}: {cause}")]
    Context {
        operation: String,
        #[source]
        cause: Box<AppError>,
    },
}

impl AppError {
    /// 递归剥到非 `Context` 的底层错误,用于按变体判断(NotFound / Validation 等)。
    pub fn root_cause(&self) -> &AppError {
        match self {
            AppError::Context { cause, .. } => cause.root_cause(),
            other => other,
        }
    }

    /// 为错误附上操作上下文,返回包装后的 [`AppError`]。
    pub fn context(self, operation: impl Into<String>) -> Self {
        AppError::Context {
            operation: operation.into(),
            cause: Box::new(self),
        }
    }
}

/// 应用程序 Result 类型
pub type AppResult<T> = Result<T, AppError>;

/// 便捷的错误创建函数
impl AppError {
    pub fn env_not_found(name: &str) -> Self {
        Self::Environment {
            message: format!("Environment not found: {name}"),
        }
    }

    pub fn config_load_failed(path: &str, reason: &str) -> Self {
        Self::Config {
            message: format!("Failed to load config file {path}: {reason}"),
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
            reason: "Path contains invalid characters".to_string(),
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

    pub fn not_found(resource: &str) -> Self {
        Self::NotFound {
            resource: resource.to_string(),
        }
    }

    pub fn validation(field: &str, reason: &str) -> Self {
        Self::Validation {
            field: field.to_string(),
            reason: reason.to_string(),
        }
    }

    pub fn config_error(message: &str) -> Self {
        Self::Config {
            message: message.to_string(),
        }
    }

    pub fn script_generation(shell_type: &str, reason: &str) -> Self {
        Self::ScriptGeneration {
            shell_type: shell_type.to_string(),
            reason: reason.to_string(),
        }
    }

    /// infra 层 String 错误的兜底转换(优先用具体变体)。
    pub fn from_string(s: &str) -> Self {
        Self::Internal {
            message: s.to_string(),
        }
    }
}

// 转换 trait 实现
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
            operation: "Thread lock failed".to_string(),
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

/// 兜底:infra 层仍返回 `String` 错误的路径(Config / installer / scanner 等)
/// 可以直接用 `?` 传播,统一收口为 [`AppError::Internal`]。
/// 关键路径应优先用具体变体(如 [`AppError::config_error`])以保留类型信息。
impl From<String> for AppError {
    fn from(message: String) -> Self {
        AppError::Internal { message }
    }
}

/// 为 Result 添加操作上下文的扩展 trait。
pub trait ResultExt<T> {
    /// 附上操作描述(如 "loading config"),便于错误定位。底层错误类型保留在
    /// [`AppError::Context`] 内,可用 [`AppError::root_cause`] 取回具体变体。
    fn with_context(self, operation: &str) -> AppResult<T>;
}

impl<T, E: Into<AppError>> ResultExt<T> for Result<T, E> {
    fn with_context(self, operation: &str) -> AppResult<T> {
        self.map_err(|e| e.into().context(operation))
    }
}
