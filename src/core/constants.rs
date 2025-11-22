//! 应用程序常量定义
//!
//! 本模块包含全局使用的常量，避免魔数并提供统一的配置值。

/// 网络相关常量
pub mod network {
    /// 默认连接超时时间（毫秒）
    pub const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 30000;
    /// 默认读取超时时间（毫秒）
    pub const DEFAULT_READ_TIMEOUT_MS: u64 = 60000;
    /// 最大重试次数
    pub const MAX_RETRY_ATTEMPTS: u32 = 3;
    /// 默认端口范围开始
    pub const MIN_PORT_NUMBER: u16 = 1024;
    /// 默认端口范围结束
    pub const MAX_PORT_NUMBER: u16 = 65535;
}

/// 缓存相关常量
pub mod cache {
    /// 默认缓存TTL（秒）
    pub const DEFAULT_CACHE_TTL: u64 = 3600; // 1小时
    /// 版本信息缓存TTL（秒）
    pub const VERSION_CACHE_TTL: u64 = 86400; // 24小时
    /// 最大缓存条目数
    pub const MAX_CACHE_ENTRIES: usize = 1000;
}

/// 文件系统相关常量
pub mod fs {
    /// 最大文件路径长度
    pub const MAX_PATH_LENGTH: usize = 260;
    /// 默认文件权限（Unix系统）
    pub const DEFAULT_FILE_PERMISSION: u32 = 0o644;
    /// 默认目录权限（Unix系统）
    pub const DEFAULT_DIR_PERMISSION: u32 = 0o755;
    /// 最大文件读取大小（字节）
    pub const MAX_FILE_READ_SIZE: usize = 10 * 1024 * 1024; // 10MB
}

/// 配置限制相关常量
pub mod config_limits {
    /// 最大环境名称长度
    pub const MAX_ENV_NAME_LENGTH: usize = 50;
    /// 最大描述长度
    pub const MAX_DESCRIPTION_LENGTH: usize = 200;
    /// 支持的最大环境数量
    pub const MAX_ENVIRONMENTS_COUNT: usize = 100;
}

/// 日志相关常量
pub mod log {
    /// 默认日志级别
    pub const DEFAULT_LOG_LEVEL: &str = "info";
    /// 日志文件最大大小（字节）
    pub const MAX_LOG_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
    /// 保留的日志文件数量
    pub const MAX_LOG_FILES: usize = 5;
}

/// 下载相关常量
pub mod download {
    /// 默认超时时间（秒）
    pub const DEFAULT_TIMEOUT: u64 = 300; // 5分钟
    /// 最大重试次数
    pub const MAX_RETRIES: u32 = 3;
    /// 重试间隔（秒）
    pub const RETRY_INTERVAL: u64 = 5;
    /// 下载缓冲区大小（字节）
    pub const BUFFER_SIZE: usize = 8192;
}

/// 环境变量相关常量
pub mod env {
    /// fnva相关的环境变量前缀
    pub const FNVA_PREFIX: &str = "FNVA_";
    /// 当前Java环境变量
    pub const CURRENT_JAVA: &str = "FNVA_CURRENT_JAVA";
    /// 当前LLM环境变量
    pub const CURRENT_LLM: &str = "FNVA_CURRENT_LLM";
    /// 当前CC环境变量
    pub const CURRENT_CC: &str = "FNVA_CURRENT_CC";
    /// 环境类型变量
    pub const ENV_TYPE: &str = "FNVA_ENV_TYPE";
}

/// 错误消息模板
pub mod error_messages {
    /// 文件不存在模板
    pub const FILE_NOT_FOUND: &str = "文件不存在: {path}";
    /// 权限不足模板
    pub const PERMISSION_DENIED: &str = "权限不足: {operation} on {path}";
    /// 配置错误模板
    pub const CONFIG_ERROR: &str = "配置错误: {details}";
    /// 网络错误模板
    pub const NETWORK_ERROR: &str = "网络错误: {operation} failed - {reason}";
    /// 验证错误模板
    pub const VALIDATION_ERROR: &str = "验证失败: {field} - {reason}";
    /// 操作失败模板
    pub const OPERATION_FAILED: &str = "操作失败: {operation} - {reason}";
}

/// 版本信息常量
pub mod version {
    /// 应用程序名称
    pub const APP_NAME: &str = "fnva";
    /// 版本号
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    /// 应用程序描述
    pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
    /// 作者信息
    pub const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
}

/// 默认配置值
pub mod defaults {
    /// 默认Shell类型
    pub const DEFAULT_SHELL: &str = "auto";
    /// 默认下载源优先级
    pub const DEFAULT_SOURCE_PRIORITY: &[&str] = &["github", "aliyun", "tsinghua"];
    /// 默认并发下载数
    pub const DEFAULT_CONCURRENT_DOWNLOADS: usize = 3;
    /// 默认日志级别
    pub const DEFAULT_LOG_LEVEL_STR: &str = "info";
    /// 默认配置目录
    pub const DEFAULT_CONFIG_DIR: &str = ".fnva";
    /// 默认缓存目录
    pub const DEFAULT_CACHE_DIR: &str = ".fnva/cache";
}

/// 正则表达式模式
pub mod patterns {
    /// Java版本匹配模式
    pub const JAVA_VERSION_PATTERN: &str = r#"^java\s+version\s+"([^"]*)""#;
    /// 语义版本匹配模式
    pub const SEMVER_PATTERN: &str = r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$";
    /// 环境名称验证模式
    pub const ENV_NAME_PATTERN: &str = r"^[a-zA-Z][a-zA-Z0-9_-]*$";
    /// URL匹配模式
    pub const URL_PATTERN: &str = r"^https?://[^\s/$.?#].[^\s]*$";
}

/// 支持的平台列表
pub const SUPPORTED_PLATFORMS: &[&str] = &["windows", "macos", "linux", "unix"];

/// 支持的架构列表
pub const SUPPORTED_ARCHITECTURES: &[&str] = &["x86_64", "x86", "arm64", "aarch64"];
