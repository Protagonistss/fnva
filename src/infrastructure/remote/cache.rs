use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs as async_fs;

/// 缓存条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub data: T,
    pub timestamp: u64,
    pub ttl: u64, // Time to live in seconds
}

impl<T> CacheEntry<T> {
    pub fn new(data: T, ttl: u64) -> Self {
        Self {
            data,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            ttl,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.timestamp) > self.ttl
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }
}

/// 版本缓存管理器
pub struct VersionCacheManager {
    cache_dir: PathBuf,
    default_ttl: u64,
}

impl VersionCacheManager {
    pub fn new() -> Result<Self, String> {
        let cache_dir = crate::infrastructure::paths::cache_dir()?;

        // 确保缓存目录存在
        fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("Failed to create cache directory: {e}"))?;

        Ok(Self {
            cache_dir,
            default_ttl: 3600, // 1 hour
        })
    }

    pub fn with_ttl(mut self, ttl: u64) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// 获取缓存文件路径
    fn cache_file_path(&self, key: &str) -> PathBuf {
        self.cache_dir.join(format!("{key}.json"))
    }

    /// 保存缓存到文件
    pub async fn save<T: Serialize>(
        &self,
        key: &str,
        data: T,
        ttl: Option<u64>,
    ) -> Result<(), String> {
        let ttl = ttl.unwrap_or(self.default_ttl);
        let entry = CacheEntry::new(data, ttl);

        let json = serde_json::to_string_pretty(&entry)
            .map_err(|e| format!("Failed to serialize cache: {e}"))?;

        let file_path = self.cache_file_path(key);
        async_fs::write(&file_path, json)
            .await
            .map_err(|e| format!("Failed to write cache file: {e}"))?;

        crate::cli::print::step("Cache", &format!("Saved {key}"));
        Ok(())
    }

    /// 从文件加载缓存
    pub async fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, String> {
        let file_path = self.cache_file_path(key);

        // 检查文件是否存在
        if !file_path.exists() {
            return Ok(None);
        }

        let json = async_fs::read_to_string(&file_path)
            .await
            .map_err(|e| format!("Failed to read cache file: {e}"))?;

        let entry: CacheEntry<T> =
            serde_json::from_str(&json).map_err(|e| format!("Failed to deserialize cache: {e}"))?;

        if entry.is_valid() {
            println!(
                "Using cache: {} ({} min remaining)",
                key,
                (entry.ttl
                    - (SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        - entry.timestamp))
                    / 60
            );
            Ok(Some(entry.data))
        } else {
            // 缓存已过期，删除文件
            async_fs::remove_file(&file_path)
                .await
                .map_err(|e| format!("Failed to remove expired cache file: {e}"))?;
            crate::cli::print::step("Cache", &format!("Expired {key}"));
            Ok(None)
        }
    }

    /// 清理所有过期缓存
    pub async fn cleanup_expired(&self) -> Result<usize, String> {
        let mut removed_count = 0;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let mut entries = async_fs::read_dir(&self.cache_dir)
            .await
            .map_err(|e| format!("Failed to read cache directory: {e}"))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to iterate cache directory: {e}"))?
        {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let json = async_fs::read_to_string(&path).await;
                if let Ok(json) = json {
                    if let Ok(entry) = serde_json::from_str::<CacheEntry<serde_json::Value>>(&json)
                    {
                        if entry.is_expired() {
                            async_fs::remove_file(&path)
                                .await
                                .map_err(|e| format!("Failed to remove expired cache file: {e}"))?;
                            removed_count += 1;
                        }
                    }
                }
            }
        }

        if removed_count > 0 {
            crate::cli::print::success(&format!("Cleaned {removed_count} expired cache files"));
        }

        Ok(removed_count)
    }

    /// 清除所有缓存
    pub async fn clear_all(&self) -> Result<(), String> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        fs::remove_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to clear cache directory: {e}"))?;

        fs::create_dir_all(&self.cache_dir)
            .map_err(|e| format!("Failed to recreate cache directory: {e}"))?;

        crate::cli::print::success("All cache cleared");
        Ok(())
    }
}

/// 缓存键生成器
pub struct CacheKeys;

impl CacheKeys {
    pub fn java_versions_tsinghua() -> String {
        "java_versions_tsinghua".to_string()
    }

    pub fn java_versions_aliyun() -> String {
        "java_versions_aliyun".to_string()
    }

    pub fn java_versions_github() -> String {
        "java_versions_github".to_string()
    }
}
