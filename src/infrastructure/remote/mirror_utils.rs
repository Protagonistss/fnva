use super::DownloadSource;
use reqwest::Client;
use std::time::Duration;

/// 从下载源中选择可用的 URL（优先主地址，失败时回退）
pub async fn pick_available_url(client: &Client, entry: &DownloadSource) -> Result<String, String> {
    if is_url_available_with_timeout(client, &entry.primary, Duration::from_secs(5)).await {
        return Ok(entry.primary.clone());
    }

    if let Some(fallback) = &entry.fallback {
        return Ok(fallback.clone());
    }

    Err("主地址和备用地址均不可用".to_string())
}

/// 检查 URL 是否可用（带超时的 HEAD 请求）
pub async fn is_url_available_with_timeout(client: &Client, url: &str, timeout: Duration) -> bool {
    match client.head(url).timeout(timeout).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// 检查 URL 是否可用（无超时，保持向后兼容）
pub async fn is_url_available(client: &Client, url: &str) -> bool {
    is_url_available_with_timeout(client, url, Duration::from_secs(5)).await
}
