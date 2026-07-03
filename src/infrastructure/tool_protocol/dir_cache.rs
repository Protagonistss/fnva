//! 目录抓取型版本发现的共用基础设施:HTTP 重试 + TTL 缓存读写。
//!
//! Java([`AdoptiumDiscovery`](crate::environments::java::version_discovery::AdoptiumDiscovery))
//! 与 Maven([`MirrorDirectoryDiscovery`](crate::environments::maven::version_discovery::MirrorDirectoryDiscovery))
//! 都从远端目录列表抓取版本,共享相同的「3 次重试 + 1s 退避」抓取和
//! `{ fetched_at, versions }` JSON 缓存格式;差异只在各自的解析逻辑。

use crate::infrastructure::tool_protocol::version_discovery::DiscoveryError;
use reqwest::Client;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const FETCH_TIMEOUT_SECS: u64 = 15;
const RETRY_DELAY_MS: u64 = 1000;
const MAX_ATTEMPTS: u32 = 3;

/// 抓取单个 URL,最多 3 次重试(1s 退避)。
///
/// - 发送失败(`send` 抛错)重试 3 次后返回 `Network` 错误;
/// - 响应体读取失败则继续重试(与原实现一致),最终返回空字符串;
/// - 成功则返回响应体文本。
pub async fn fetch_with_retry(client: &Client, url: &str) -> Result<String, DiscoveryError> {
    let mut attempts = 0;
    let mut text = String::new();
    while attempts < MAX_ATTEMPTS {
        attempts += 1;
        match client
            .get(url)
            .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(t) = resp.text().await {
                    text = t;
                    break;
                }
                // 响应体读取失败:原实现直接进入下一轮(无延迟)
            }
            Err(e) => {
                if attempts >= MAX_ATTEMPTS {
                    return Err(DiscoveryError::Network(format!(
                        "Failed to fetch {url} after {MAX_ATTEMPTS} attempts: {e}"
                    )));
                }
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
            }
        }
    }
    Ok(text)
}

/// TTL 缓存条目:序列化为 `{ fetched_at, versions }` JSON。
#[derive(Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub fetched_at: i64,
    pub versions: Vec<T>,
}

impl<T: Serialize + DeserializeOwned + Clone> CacheEntry<T> {
    /// 读取并校验 TTL;文件缺失、损坏或过期均返回 `None`。
    pub fn read(path: &Path, ttl_secs: i64) -> Option<Vec<T>> {
        let content = std::fs::read_to_string(path).ok()?;
        let entry: CacheEntry<T> = serde_json::from_str(&content).ok()?;
        if chrono::Utc::now().timestamp() - entry.fetched_at < ttl_secs {
            Some(entry.versions)
        } else {
            None
        }
    }

    /// 写入缓存:自动创建父目录,任何 IO/序列化失败都静默(缓存只是加速,非必需)。
    pub fn write(path: &Path, versions: &[T]) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let entry = CacheEntry {
            fetched_at: chrono::Utc::now().timestamp(),
            versions: versions.to_vec(),
        };
        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = std::fs::write(path, json);
        }
    }
}
