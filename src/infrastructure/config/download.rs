use serde::{Deserialize, Serialize};

/// 下载配置
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DownloadConfig {
    /// 重试次数
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    /// 初始重试延迟（毫秒）
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
    /// 是否使用指数退避
    #[serde(default = "default_exponential_backoff")]
    pub exponential_backoff: bool,
    /// 连接超时时间（秒）
    #[serde(default = "default_connect_timeout_sec")]
    pub connect_timeout_sec: u64,
    /// 读取超时时间（秒）
    #[serde(default = "default_read_timeout_sec")]
    pub read_timeout_sec: u64,
}

fn default_retry_count() -> u32 {
    3
}

fn default_retry_delay_ms() -> u64 {
    1000
}

fn default_exponential_backoff() -> bool {
    true
}

fn default_connect_timeout_sec() -> u64 {
    30
}

fn default_read_timeout_sec() -> u64 {
    300
}
