//! 标准化错误消息模块
//!
//! 本模块提供统一的错误消息格式和多语言支持。


/// 错误消息语言
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Chinese,
    English,
}

/// 标准化错误消息
#[derive(Debug, Clone)]
pub struct ErrorMessage {
    /// 错误代码
    pub code: &'static str,
    /// 中文消息
    pub chinese: &'static str,
    /// 英文消息
    pub english: &'static str,
    /// 建议的解决方案
    pub suggestions: &'static [&'static str],
    /// 帮助URL
    pub help_url: Option<&'static str>,
}

impl ErrorMessage {
    /// 获取指定语言的消息
    pub fn message(&self, language: Language) -> &str {
        match language {
            Language::Chinese => self.chinese,
            Language::English => self.english,
        }
    }

    /// 获取建议列表
    pub fn suggestions(&self) -> &[&'static str] {
        self.suggestions
    }

    /// 获取帮助URL
    pub fn help_url(&self) -> Option<&str> {
        self.help_url
    }
}

/// 格式化错误消息
pub struct ErrorMessageFormatter {
    language: Language,
}

impl ErrorMessageFormatter {
    /// 创建新的格式化器
    pub fn new(language: Language) -> Self {
        Self { language }
    }

    /// 格式化带参数的错误消息
    pub fn format(&self, template: &str, args: &[&str]) -> String {
        let mut result = template.to_string();
        for (i, arg) in args.iter().enumerate() {
            result = result.replace(&format!("{{{}}}", i), arg);
        }
        result
    }

    /// 格式化环境相关错误
    pub fn format_env_error(&self, env_type: &str, env_name: &str, details: &str) -> String {
        match self.language {
            Language::Chinese => {
                format!("{}环境 '{}' 操作失败: {}", env_type, env_name, details)
            }
            Language::English => {
                format!(
                    "Failed to {} '{}' environment: {}",
                    env_type, env_name, details
                )
            }
        }
    }

    /// 格式化网络相关错误
    pub fn format_network_error(&self, operation: &str, url: &str, details: &str) -> String {
        match self.language {
            Language::Chinese => {
                format!(
                    "网络请求失败 - 操作: {}, URL: {}, 详情: {}",
                    operation, url, details
                )
            }
            Language::English => {
                format!(
                    "Network request failed - operation: {}, URL: {}, details: {}",
                    operation, url, details
                )
            }
        }
    }

    /// 格式化文件系统错误
    pub fn format_fs_error(&self, operation: &str, path: &str, details: &str) -> String {
        match self.language {
            Language::Chinese => {
                format!(
                    "文件系统错误 - 操作: {}, 路径: {}, 详情: {}",
                    operation, path, details
                )
            }
            Language::English => {
                format!(
                    "File system error - operation: {}, path: {}, details: {}",
                    operation, path, details
                )
            }
        }
    }

    /// 格式化配置相关错误
    pub fn format_config_error(&self, section: &str, details: &str) -> String {
        match self.language {
            Language::Chinese => {
                format!("配置错误 - 节: {}, 详情: {}", section, details)
            }
            Language::English => {
                format!(
                    "Configuration error - section: {}, details: {}",
                    section, details
                )
            }
        }
    }

    /// 格式化验证相关错误
    pub fn format_validation_error(&self, field: &str, value: &str, rule: &str) -> String {
        match self.language {
            Language::Chinese => {
                format!(
                    "验证失败 - 字段: '{}' (值: '{}'), 规则: {}",
                    field, value, rule
                )
            }
            Language::English => {
                format!(
                    "Validation failed - field: '{}' (value: '{}'), rule: {}",
                    field, value, rule
                )
            }
        }
    }
}

impl Default for ErrorMessageFormatter {
    fn default() -> Self {
        Self::new(Language::Chinese) // 默认使用中文
    }
}

/// 常用错误消息定义
pub mod messages {
    use super::ErrorMessage;

    /// 环境不存在
    pub const ENV_NOT_FOUND: ErrorMessage = ErrorMessage {
        code: "ENV_001",
        chinese: "环境不存在",
        english: "Environment not found",
        suggestions: &["检查环境名称是否正确", "使用 'env list' 查看可用环境"],
        help_url: None,
    };

    /// 环境已存在
    pub const ENV_ALREADY_EXISTS: ErrorMessage = ErrorMessage {
        code: "ENV_002",
        chinese: "环境已存在",
        english: "Environment already exists",
        suggestions: &["使用不同的环境名称", "使用 'env remove' 删除现有环境"],
        help_url: None,
    };

    /// 环境验证失败
    pub const ENV_VALIDATION_FAILED: ErrorMessage = ErrorMessage {
        code: "ENV_003",
        chinese: "环境验证失败",
        english: "Environment validation failed",
        suggestions: &["检查环境路径是否正确", "确认环境是否可访问"],
        help_url: None,
    };

    /// 配置文件不存在
    pub const CONFIG_NOT_FOUND: ErrorMessage = ErrorMessage {
        code: "CONFIG_001",
        chinese: "配置文件不存在",
        english: "Configuration file not found",
        suggestions: &["使用 'init' 命令创建配置", "检查配置文件路径"],
        help_url: None,
    };

    /// 配置文件格式错误
    pub const CONFIG_FORMAT_ERROR: ErrorMessage = ErrorMessage {
        code: "CONFIG_002",
        chinese: "配置文件格式错误",
        english: "Configuration file format error",
        suggestions: &["检查配置文件语法", "使用 'validate' 命令验证配置"],
        help_url: None,
    };

    /// 网络连接失败
    pub const NETWORK_CONNECTION_FAILED: ErrorMessage = ErrorMessage {
        code: "NET_001",
        chinese: "网络连接失败",
        english: "Network connection failed",
        suggestions: &["检查网络连接", "尝试使用代理", "检查防火墙设置"],
        help_url: None,
    };

    /// 下载失败
    pub const DOWNLOAD_FAILED: ErrorMessage = ErrorMessage {
        code: "NET_002",
        chinese: "下载失败",
        english: "Download failed",
        suggestions: &["检查网络连接", "尝试其他下载源", "检查磁盘空间"],
        help_url: None,
    };

    /// 权限不足
    pub const PERMISSION_DENIED: ErrorMessage = ErrorMessage {
        code: "FS_001",
        chinese: "权限不足",
        english: "Permission denied",
        suggestions: &["使用管理员权限运行", "检查文件/目录权限"],
        help_url: None,
    };

    /// 文件不存在
    pub const FILE_NOT_FOUND: ErrorMessage = ErrorMessage {
        code: "FS_002",
        chinese: "文件不存在",
        english: "File not found",
        suggestions: &["检查文件路径", "确认文件是否已创建"],
        help_url: None,
    };

    /// 磁盘空间不足
    pub const INSUFFICIENT_SPACE: ErrorMessage = ErrorMessage {
        code: "FS_003",
        chinese: "磁盘空间不足",
        english: "Insufficient disk space",
        suggestions: &["清理磁盘空间", "选择其他安装路径"],
        help_url: None,
    };

    /// 无效参数
    pub const INVALID_ARGUMENT: ErrorMessage = ErrorMessage {
        code: "ARG_001",
        chinese: "无效参数",
        english: "Invalid argument",
        suggestions: &["查看命令帮助", "检查参数格式"],
        help_url: None,
    };

    /// 不支持的操作
    pub const UNSUPPORTED_OPERATION: ErrorMessage = ErrorMessage {
        code: "OP_001",
        chinese: "不支持的操作",
        english: "Unsupported operation",
        suggestions: &["检查操作是否支持", "查看文档获取更多信息"],
        help_url: None,
    };
}

/// 错误消息构建器
pub struct ErrorMessageBuilder {
    code: &'static str,
    chinese: &'static str,
    english: &'static str,
    suggestions: Vec<&'static str>,
    help_url: Option<&'static str>,
}

impl ErrorMessageBuilder {
    /// 创建新的错误消息构建器
    pub fn new(code: &'static str, chinese: &'static str, english: &'static str) -> Self {
        Self {
            code,
            chinese,
            english,
            suggestions: Vec::new(),
            help_url: None,
        }
    }

    /// 添加建议
    pub fn suggestion(mut self, suggestion: &'static str) -> Self {
        self.suggestions.push(suggestion);
        self
    }

    /// 设置帮助URL
    pub fn help_url(mut self, url: &'static str) -> Self {
        self.help_url = Some(url);
        self
    }

    /// 构建错误消息
    pub fn build(self) -> ErrorMessage {
        let suggestions = self.suggestions.leak();
        ErrorMessage {
            code: self.code,
            chinese: self.chinese,
            english: self.english,
            suggestions,
            help_url: self.help_url,
        }
    }
}

/// 用于格式化错误消息的宏
#[macro_export]
macro_rules! error_msg {
    ($code:expr, $chinese:expr, $english:expr) => {
        $crate::core::error_messages::ErrorMessageBuilder::new($code, $chinese, $english)
    };
    ($code:expr, $chinese:expr, $english:expr, suggestion = $suggestion:expr) => {
        $crate::error_msg!($code, $chinese, $english).suggestion($suggestion)
    };
    ($code:expr, $chinese:expr, $english:expr, $($suggestion:expr),+) => {
        $crate::error_msg!($code, $chinese, $english)
            $(.suggestion($suggestion))+
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_message_localization() {
        let formatter = ErrorMessageFormatter::new(Language::Chinese);
        let msg = formatter.format_env_error("Java", "jdk17", "路径不存在");
        assert!(msg.contains("Java"));
        assert!(msg.contains("jdk17"));
        assert!(msg.contains("路径不存在"));
    }

    #[test]
    fn test_error_message_builder() {
        let msg = ErrorMessageBuilder::new("TEST_001", "测试错误", "Test error")
            .suggestion("建议1")
            .suggestion("建议2")
            .help_url("https://example.com")
            .build();

        assert_eq!(msg.code, "TEST_001");
        assert_eq!(msg.suggestions().len(), 2);
        assert_eq!(msg.help_url(), Some("https://example.com"));
    }
}
