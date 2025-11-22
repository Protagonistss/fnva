use futures_util::StreamExt;
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};
use std::path::Path;

/// 错误类型：用于区分临时错误和永久错误
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    /// 临时错误（网络问题、超时等，可以重试）
    Transient(String),
    /// 永久错误（404、403等，不应重试）
    Permanent(String),
}

/// 下载选项
#[derive(Clone)]
pub struct DownloadOptions {
    pub expected_sha256: Option<String>,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
    pub exponential_backoff: bool,
    pub connect_timeout_sec: u64,
    pub read_timeout_sec: u64,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            expected_sha256: None,
            retry_count: 3,
            retry_delay_ms: 1000,
            exponential_backoff: true,
            connect_timeout_sec: 30,
            read_timeout_sec: 300,
        }
    }
}

impl DownloadOptions {
    /// 从配置创建下载选项
    pub fn from_config(config: &crate::infrastructure::config::DownloadConfig) -> Self {
        Self {
            expected_sha256: None,
            retry_count: config.retry_count,
            retry_delay_ms: config.retry_delay_ms,
            exponential_backoff: config.exponential_backoff,
            connect_timeout_sec: config.connect_timeout_sec,
            read_timeout_sec: config.read_timeout_sec,
        }
    }

    /// 计算重试延迟（支持指数退避）
    fn calculate_retry_delay(&self, attempt: u32) -> u64 {
        if self.exponential_backoff {
            // 指数退避：delay * 2^(attempt-1)，最大不超过 60 秒
            let delay = self.retry_delay_ms * 2_u64.pow(attempt.saturating_sub(1));
            delay.min(60000)
        } else {
            self.retry_delay_ms
        }
    }
}

/// 判断错误类型
fn classify_error(error: &str, status_code: Option<u16>) -> ErrorType {
    // 根据状态码判断
    if let Some(code) = status_code {
        match code {
            404 | 403 | 401 => return ErrorType::Permanent(format!("资源不存在或无权访问 (HTTP {})", code)),
            500..=599 => return ErrorType::Transient(format!("服务器错误 (HTTP {})", code)),
            _ => {}
        }
    }

    // 根据错误消息判断
    let error_lower = error.to_lowercase();
    if error_lower.contains("not found") || error_lower.contains("404") {
        ErrorType::Permanent("资源未找到".to_string())
    } else if error_lower.contains("timeout") || error_lower.contains("timed out") {
        ErrorType::Transient("连接超时".to_string())
    } else if error_lower.contains("network") || error_lower.contains("connection") {
        ErrorType::Transient("网络连接问题".to_string())
    } else if error_lower.contains("dns") || error_lower.contains("resolve") {
        ErrorType::Transient("DNS 解析失败".to_string())
    } else {
        ErrorType::Transient(error.to_string())
    }
}

/// 验证数据哈希
fn verify_sha256(data: &[u8], expected: &str) -> Result<(), String> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let actual = hex::encode(result);
    
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!("SHA256 mismatch: expected {}, got {}", expected, actual))
    }
}

/// 验证文件哈希
async fn verify_file_sha256(path: &Path, expected: &str) -> Result<(), String> {
    let mut file = tokio::fs::File::open(path).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192]; // 8KB buffer

    use tokio::io::AsyncReadExt;
    loop {
        let n = file.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[0..n]);
    }

    let result = hasher.finalize();
    let actual = hex::encode(result);

    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!("SHA256 mismatch: expected {}, got {}", expected, actual))
    }
}

/// 从配置加载下载选项
pub fn load_download_options() -> DownloadOptions {
    crate::infrastructure::config::Config::load()
        .map(|config| DownloadOptions::from_config(&config.download))
        .unwrap_or_else(|_| DownloadOptions::default())
}

/// 通用的下载工具：流式下载并回调进度，支持重试和校验。
pub async fn download_to_bytes(
    client: &Client,
    url: &str,
    progress: impl Fn(u64, u64),
) -> Result<Vec<u8>, String> {
    download_to_bytes_with_options(client, url, progress, DownloadOptions::default()).await
}

pub async fn download_to_bytes_with_options(
    client: &Client,
    url: &str,
    progress: impl Fn(u64, u64),
    options: DownloadOptions,
) -> Result<Vec<u8>, String> {
    let mut attempts = 0;
    let mut last_status_code: Option<u16> = None;

    loop {
        attempts += 1;
        match download_to_bytes_internal(client, url, &progress).await {
            Ok(data) => {
                if let Some(expected) = &options.expected_sha256 {
                    if let Err(e) = verify_sha256(&data, expected) {
                        println!("⚠️  校验失败 (尝试 {}/{}): {}", attempts, options.retry_count + 1, e);
                        if attempts > options.retry_count {
                            return Err(format!("校验失败 (已重试 {} 次): {}", options.retry_count, e));
                        }
                        let delay = options.calculate_retry_delay(attempts);
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    println!("✅ SHA256 校验通过");
                }
                return Ok(data);
            }
            Err(e) => {
                // 尝试从错误消息中提取状态码
                if e.contains("状态码:") {
                    if let Some(code_str) = e.split("状态码:").nth(1) {
                        if let Ok(code) = code_str.trim().split_whitespace().next().unwrap_or("").parse::<u16>() {
                            last_status_code = Some(code);
                        }
                    }
                }

                let error_type = classify_error(&e, last_status_code);
                
                // 永久错误不重试
                if matches!(error_type, ErrorType::Permanent(_)) {
                    return Err(format!("{}: {}", 
                        if let ErrorType::Permanent(msg) = error_type { msg } else { unreachable!() },
                        e));
                }

                if attempts > options.retry_count {
                    return Err(format!("下载失败 (已重试 {} 次): {}。URL: {}", 
                        options.retry_count, 
                        e,
                        url));
                }

                let delay = options.calculate_retry_delay(attempts);
                println!("⚠️  下载出错 (尝试 {}/{}): {}。{}ms 后重试...", 
                    attempts, 
                    options.retry_count + 1, 
                    e,
                    delay);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }
    }
}

async fn download_to_bytes_internal(
    client: &Client,
    url: &str,
    progress: &impl Fn(u64, u64),
) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .header("User-Agent", "fnva/0.0.5")
        .send()
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("timeout") {
                format!("连接超时: {}", error_msg)
            } else if error_msg.contains("dns") || error_msg.contains("resolve") {
                format!("DNS 解析失败: {}", error_msg)
            } else {
                format!("网络请求失败: {} (URL: {})", error_msg, url)
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("服务器返回状态码: {} (URL: {})", status, url));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut data = Vec::new();
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取数据失败: {}", e))?;
        downloaded += chunk.len() as u64;
        progress(downloaded, total_size);
        data.extend_from_slice(&chunk);
    }

    Ok(data)
}

pub async fn download_to_file(
    client: &Client,
    url: &str,
    file_path: &Path,
    progress: impl Fn(u64, u64),
) -> Result<(), String> {
    let options = load_download_options();
    download_to_file_with_options(client, url, file_path, progress, options).await
}

pub async fn download_to_file_with_options(
    client: &Client,
    url: &str,
    file_path: &Path,
    progress: impl Fn(u64, u64),
    options: DownloadOptions,
) -> Result<(), String> {
    let mut attempts = 0;
    let mut last_status_code: Option<u16> = None;

    loop {
        attempts += 1;
        match download_to_file_internal(client, url, file_path, &progress).await {
            Ok(_) => {
                if let Some(expected) = &options.expected_sha256 {
                    if let Err(e) = verify_file_sha256(file_path, expected).await {
                        println!("⚠️  文件校验失败 (尝试 {}/{}): {}", attempts, options.retry_count + 1, e);
                        // 删除损坏的文件
                        let _ = tokio::fs::remove_file(file_path).await;
                        
                        if attempts > options.retry_count {
                            return Err(format!("校验失败 (已重试 {} 次): {}", options.retry_count, e));
                        }
                        let delay = options.calculate_retry_delay(attempts);
                        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    println!("✅ 文件 SHA256 校验通过");
                }
                return Ok(());
            }
            Err(e) => {
                // 尝试从错误消息中提取状态码
                if e.contains("状态码:") {
                    if let Some(code_str) = e.split("状态码:").nth(1) {
                        if let Ok(code) = code_str.trim().split_whitespace().next().unwrap_or("").parse::<u16>() {
                            last_status_code = Some(code);
                        }
                    }
                }

                // 尝试删除可能未完成的文件
                let _ = tokio::fs::remove_file(file_path).await;
                
                let error_type = classify_error(&e, last_status_code);
                
                // 永久错误不重试
                if matches!(error_type, ErrorType::Permanent(_)) {
                    return Err(format!("{}: {} (URL: {})", 
                        if let ErrorType::Permanent(msg) = error_type { msg } else { unreachable!() },
                        e,
                        url));
                }

                if attempts > options.retry_count {
                    return Err(format!("下载失败 (已重试 {} 次): {}。URL: {}，文件: {}", 
                        options.retry_count, 
                        e,
                        url,
                        file_path.display()));
                }

                let delay = options.calculate_retry_delay(attempts);
                println!("⚠️  下载出错 (尝试 {}/{}): {}。{}ms 后重试...", 
                    attempts, 
                    options.retry_count + 1, 
                    e,
                    delay);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }
    }
}

async fn download_to_file_internal(
    client: &Client,
    url: &str,
    file_path: &Path,
    progress: &impl Fn(u64, u64),
) -> Result<(), String> {
    let response = client
        .get(url)
        .header("User-Agent", "fnva/0.0.5")
        .send()
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("timeout") {
                format!("连接超时: {}", error_msg)
            } else if error_msg.contains("dns") || error_msg.contains("resolve") {
                format!("DNS 解析失败: {}", error_msg)
            } else {
                format!("网络请求失败: {} (URL: {})", error_msg, url)
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(format!("服务器返回状态码: {} (URL: {})", status, url));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    
    // 使用临时文件
    let temp_path = file_path.with_extension("downloading");
    let mut file = tokio::fs::File::create(&temp_path)
        .await
        .map_err(|e| format!("创建文件失败: {}", e))?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取数据失败: {}", e))?;
        downloaded += chunk.len() as u64;
        progress(downloaded, total_size);
        file.write_all(&chunk).await.map_err(|e| format!("写入文件失败: {}", e))?;
    }

    file.flush().await.map_err(|e| format!("刷新文件失败: {}", e))?;
    drop(file); // 关闭文件

    // 重命名为目标文件
    tokio::fs::rename(&temp_path, file_path)
        .await
        .map_err(|e| format!("重命名文件失败: {}", e))?;

    Ok(())
}
