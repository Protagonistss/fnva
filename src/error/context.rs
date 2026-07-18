use crate::error::{AppError, AppResult};
use std::sync::{Arc, Mutex};

/// 提供安全的 Mutex 操作,避免 unwrap()
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

    /// 安全地获取锁,如果锁定失败返回错误
    pub fn lock(&self) -> AppResult<std::sync::MutexGuard<'_, T>> {
        self.inner
            .lock()
            .map_err(|_| AppError::lock_failed(&format!("Failed to lock: {}", self.name)))
    }

    /// 尝试获取锁,非阻塞
    pub fn try_lock(&self) -> AppResult<std::sync::MutexGuard<'_, T>> {
        self.inner
            .try_lock()
            .map_err(|_| AppError::lock_failed(&format!("Unable to acquire lock: {}", self.name)))
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

/// 提供安全的路径转换,避免 unwrap()
pub fn safe_path_to_str(path: &std::path::Path) -> Result<&str, AppError> {
    path.to_str()
        .ok_or_else(|| AppError::path_conversion_failed(&format!("{path:?}")))
}

/// 提供安全的路径字符串转换
pub fn safe_path_to_string(path: &std::path::Path) -> Result<String, AppError> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::path_conversion_failed(&format!("{path:?}")))
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

/// 为 Option 添加上下文信息:`None` 时把 `error` 包装上 `operation` 描述后返回。
pub fn option_with_context<T>(option: Option<T>, error: AppError, operation: &str) -> AppResult<T> {
    option.ok_or_else(|| error.context(operation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ResultExt;

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
    fn test_with_context_wraps_error() {
        let result: Result<i32, AppError> = Err(AppError::Internal {
            message: "test error".to_string(),
        });
        let wrapped = result.with_context("test operation");
        assert!(matches!(wrapped, Err(AppError::Context { .. })));
        assert!(matches!(
            wrapped.unwrap_err().root_cause(),
            AppError::Internal { .. }
        ));
    }

    #[test]
    fn test_root_cause_unwraps_nested_context() {
        let inner = AppError::not_found("java env");
        let wrapped = inner.context("switching").context("higher");
        assert!(matches!(wrapped.root_cause(), AppError::NotFound { .. }));
    }
}
