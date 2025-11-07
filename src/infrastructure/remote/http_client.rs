use reqwest::{Client, Response};
use std::time::Duration;

/// HTTP 客户端包装器
pub struct HttpClient {
    client: Client,
    #[allow(dead_code)]
    timeout: Duration,
}

impl HttpClient {
    /// 创建新的 HTTP 客户端
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let timeout = Duration::from_secs(30);
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("fnva/0.0.4")
            .build()?;

        Ok(Self { client, timeout })
    }

    /// 创建带自定义超时的 HTTP 客户端
    pub fn with_timeout(timeout_secs: u64) -> Result<Self, Box<dyn std::error::Error>> {
        let timeout = Duration::from_secs(timeout_secs);
        let client = Client::builder()
            .timeout(timeout)
            .user_agent("fnva/0.0.4")
            .build()?;

        Ok(Self { client, timeout })
    }

    /// GET 请求
    pub async fn get(&self, url: &str) -> Result<Response, Box<dyn std::error::Error>> {
        let response = self.client.get(url).send().await?;
        Ok(response)
    }

    /// GET 请求并返回文本
    pub async fn get_text(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.get(url).await?;
        let text = response.text().await?;
        Ok(text)
    }

    /// GET 请求并返回 JSON
    pub async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        let response = self.get(url).await?;
        let json = response.json().await?;
        Ok(json)
    }

    /// 下载文件
    pub async fn download(
        &self,
        url: &str,
        progress_callback: Option<Box<dyn Fn(u64, u64) + Send + Sync>>,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let response = self.get(url).await?;
        let total_size = response.content_length().unwrap_or(0);

        let mut data = Vec::new();
        let mut downloaded = 0u64;

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            downloaded += chunk.len() as u64;
            data.extend_from_slice(&chunk);

            if let Some(ref callback) = progress_callback {
                callback(downloaded, total_size);
            }
        }

        Ok(data)
    }

    /// 检查 URL 是否可访问
    pub async fn check_url(&self, url: &str) -> Result<bool, Box<dyn std::error::Error>> {
        match self.client.head(url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(_) => Ok(false),
        }
    }

    /// 获取重定向后的最终 URL
    pub async fn get_final_url(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = self.client.get(url).send().await?;
        let final_url = response.url().clone();
        Ok(final_url.to_string())
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create HTTP client")
    }
}

/// 网络错误类型
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Request timeout")]
    Timeout,

    #[error("Network unavailable")]
    NetworkUnavailable,

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Not found")]
    NotFound,

    #[error("Generic error: {0}")]
    GenericError(String),
}

impl NetworkError {
    /// 从 HTTP 状态码创建错误
    pub fn from_status(status: reqwest::StatusCode) -> Self {
        match status {
            reqwest::StatusCode::NOT_FOUND => Self::NotFound,
            reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                Self::ServerError("Internal server error".to_string())
            }
            reqwest::StatusCode::SERVICE_UNAVAILABLE => {
                Self::ServerError("Service unavailable".to_string())
            }
            _ => Self::ServerError(format!("HTTP error: {}", status)),
        }
    }
}