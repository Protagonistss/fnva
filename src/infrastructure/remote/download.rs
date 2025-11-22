use futures_util::StreamExt;
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use sha2::{Sha256, Digest};
use std::path::Path;

/// 下载选项
#[derive(Clone)]
pub struct DownloadOptions {
    pub expected_sha256: Option<String>,
    pub retry_count: u32,
    pub retry_delay_ms: u64,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            expected_sha256: None,
            retry_count: 3,
            retry_delay_ms: 1000,
        }
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
    loop {
        attempts += 1;
        match download_to_bytes_internal(client, url, &progress).await {
            Ok(data) => {
                if let Some(expected) = &options.expected_sha256 {
                    if let Err(e) = verify_sha256(&data, expected) {
                        println!("⚠️  校验失败 (尝试 {}/{}): {}", attempts, options.retry_count + 1, e);
                        if attempts > options.retry_count {
                            return Err(e);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(options.retry_delay_ms)).await;
                        continue;
                    }
                    println!("✅ SHA256 校验通过");
                }
                return Ok(data);
            }
            Err(e) => {
                if attempts > options.retry_count {
                    return Err(format!("下载失败 (已重试 {} 次): {}", options.retry_count, e));
                }
                println!("⚠️  下载出错 (尝试 {}/{}): {}. {}ms 后重试...", attempts, options.retry_count + 1, e, options.retry_delay_ms);
                tokio::time::sleep(std::time::Duration::from_millis(options.retry_delay_ms)).await;
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
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("服务器返回状态码: {}", response.status()));
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
    download_to_file_with_options(client, url, file_path, progress, DownloadOptions::default()).await
}

pub async fn download_to_file_with_options(
    client: &Client,
    url: &str,
    file_path: &Path,
    progress: impl Fn(u64, u64),
    options: DownloadOptions,
) -> Result<(), String> {
    let mut attempts = 0;
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
                            return Err(e);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(options.retry_delay_ms)).await;
                        continue;
                    }
                    println!("✅ 文件 SHA256 校验通过");
                }
                return Ok(());
            }
            Err(e) => {
                // 尝试删除可能未完成的文件
                let _ = tokio::fs::remove_file(file_path).await;
                
                if attempts > options.retry_count {
                    return Err(format!("下载失败 (已重试 {} 次): {}", options.retry_count, e));
                }
                println!("⚠️  下载出错 (尝试 {}/{}): {}. {}ms 后重试...", attempts, options.retry_count + 1, e, options.retry_delay_ms);
                tokio::time::sleep(std::time::Duration::from_millis(options.retry_delay_ms)).await;
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
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("服务器返回状态码: {}", response.status()));
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
