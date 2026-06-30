use reqwest::Client;
use std::time::Duration;

/// Check if a URL is available (HEAD request with timeout).
pub async fn is_url_available_with_timeout(client: &Client, url: &str, timeout: Duration) -> bool {
    match client.head(url).timeout(timeout).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Check if a URL is available (HEAD request, 5s default timeout).
pub async fn is_url_available(client: &Client, url: &str) -> bool {
    is_url_available_with_timeout(client, url, Duration::from_secs(5)).await
}
