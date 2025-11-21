use futures_util::StreamExt;
use reqwest::Client;
use tokio::io::AsyncWriteExt;

/// 通用的下载工具：流式下载并回调进度。
pub async fn download_to_bytes(
    client: &Client,
    url: &str,
    progress: impl Fn(u64, u64),
) -> Result<Vec<u8>, String> {
    let response = client
        .get(url)
        .header("User-Agent", "fnva/0.0.5")
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("下载失败: {}", response.status()));
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
    file_path: &std::path::Path,
    progress: impl Fn(u64, u64),
) -> Result<(), String> {
    let response = client
        .get(url)
        .header("User-Agent", "fnva/0.0.5")
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("下载失败: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();
    let mut file = tokio::fs::File::create(file_path)
        .await
        .map_err(|e| format!("创建文件失败: {}", e))?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("读取数据失败: {}", e))?;
        downloaded += chunk.len() as u64;
        progress(downloaded, total_size);
        file.write_all(&chunk).await.map_err(|e| format!("写入文件失败: {}", e))?;
    }

    file.flush().await.map_err(|e| format!("刷新文件失败: {}", e))?;

    Ok(())
}
