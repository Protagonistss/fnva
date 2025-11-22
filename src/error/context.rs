use std::sync::{Arc, Mutex};
use crate::error::{AppError, ContextualError, ErrorContext};

/// 提供安全的 Mutex 操作，避免 unwrap()
pub struct SafeMutex<T> {
    inner: Arc<Mutex<T>>,
    name: String,
}

impl<T> SafeMutex<T> {
    pub fn new(data: T, name: &str) -> Self {
        Self {
            inner: Arc::new(Mutex::new(data)),
            name: name.to_string(),
        }
    }

    /// 安全地获取锁，如果锁定失败返回错误
    pub fn lock(&self) -> Result<std::sync::MutexGuard<T>, ContextualError> {
        self.inner.lock().map_err(|_| {
            ContextualError {
                error: AppError::lock_failed(&format!("锁定失败: {}", self.name)),
                context: ErrorContext {
                    operation: format!("获取 {} 锁时发生死锁", self.name),
                    suggestions: vec![
                        "检查是否存在死锁".to_string(),
                        "确保其他线程正确释放锁".to_string(),
                    ],
                    help_url: None,
                },
            }
        })
    }

    /// 尝试获取锁，非阻塞
    pub fn try_lock(&self) -> Result<std::sync::MutexGuard<T>, ContextualError> {
        self.inner.try_lock().map_err(|_| {
            ContextualError {
                error: AppError::lock_failed(&format!("无法获取锁: {}", self.name)),
                context: ErrorContext {
                    operation: format!("尝试获取 {} 锁时被占用", self.name),
                    suggestions: vec![
                        "稍后重试".to_string(),
                        "检查锁的持有者".to_string(),
                    ],
                    help_url: None,
                },
            }
        })
    }
}

impl<T> Clone for SafeMutex<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            name: self.name.clone(),
        }
    }
}

/// 提供安全的路径转换，避免 unwrap()
pub fn safe_path_to_str(path: &std::path::Path) -> Result<&str, AppError> {
    path.to_str().ok_or_else(|| {
        AppError::path_conversion_failed(&format!("{:?}", path))
    })
}

/// 提供安全的路径字符串转换
pub fn safe_path_to_string(path: &std::path::Path) -> Result<String, AppError> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            AppError::path_conversion_failed(&format!("{:?}", path))
        })
}

/// 安全的 JSON 序列化
pub fn safe_to_json_pretty<T: serde::Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string_pretty(value).map_err(Into::into)
}

/// 安全的 JSON 序列化（紧凑格式）
pub fn safe_to_json<T: serde::Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(Into::into)
}

/// 安全的 JSON 反序列化
pub fn safe_from_json<T: for<'de> serde::Deserialize<'de>>(json: &str) -> Result<T, AppError> {
    serde_json::from_str(json).map_err(Into::into)
}

/// 为Result添加上下文信息的辅助函数
pub fn with_context<T, E: Into<AppError>>(
    result: Result<T, E>,
    operation: &str,
) -> Result<T, ContextualError> {
    result.map_err(|e| ContextualError {
        error: e.into(),
        context: ErrorContext {
            operation: operation.to_string(),
            suggestions: Vec::new(),
            help_url: None,
        },
    })
}

/// 为Option添加上下文信息的辅助函数
pub fn option_with_context<T>(
    option: Option<T>,
    error: AppError,
    operation: &str,
) -> Result<T, ContextualError> {
    option.ok_or_else(|| ContextualError {
        error,
        context: ErrorContext {
            operation: operation.to_string(),
            suggestions: Vec::new(),
            help_url: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_mutex() {
        let mutex = SafeMutex::new(42, "test");
        let guard = mutex.lock().unwrap();
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_safe_path_conversion() {
        use std::path::Path;

        let path = Path::new("/valid/path");
        assert!(safe_path_to_str(path).is_ok());

        // 注意：实际测试无效UTF-8路径在Rust中比较复杂，这里不进行
    }

    #[test]
    fn test_safe_json_serialization() {
        let data = serde_json::json!({"key": "value"});
        assert!(safe_to_json_pretty(&data).is_ok());
    }

    #[test]
    fn test_with_context() {
        let result: Result<i32, &str> = Err("test error");
        let contextual_result = with_context(result, "test operation");
        assert!(contextual_result.is_err());
    }
}