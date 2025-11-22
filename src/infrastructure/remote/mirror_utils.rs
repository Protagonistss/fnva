use super::DownloadSource;
use reqwest::Client;

/// 从下载源中选择可用的 URL（优先主地址，失败时回退）
pub async fn pick_available_url(client: &Client, entry: &DownloadSource) -> Result<String, String> {
    // 优先使用主地址
    if is_url_available(client, &entry.primary).await {
        return Ok(entry.primary.clone());
    }

    // 如果主地址不可用，尝试备用地址
    if let Some(fallback) = &entry.fallback {
        return Ok(fallback.clone());
    }

    Err("主地址和备用地址均不可用".to_string())
}

/// 检查 URL 是否可用（通过 HEAD 请求）
pub async fn is_url_available(client: &Client, url: &str) -> bool {
    match client.head(url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}
