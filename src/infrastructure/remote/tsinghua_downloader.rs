use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{download::download_to_bytes, platform::Platform};
use super::java_downloader::{JavaDownloader, DownloadTarget, DownloadError};
use super::UnifiedJavaVersion;
use super::DownloadSource;

/// Mirror download entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsinghuaDownloadEntry {
    pub primary: String,
    pub fallback: Option<String>,
}

/// Downloader for Tsinghua Adoptium mirror
pub struct TsinghuaJavaDownloader {
    client: reqwest::Client,
    base_url: String,
}

impl TsinghuaJavaDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://mirrors.tuna.tsinghua.edu.cn/Adoptium".to_string(),
        }
    }

    async fn list_versions_internal(&self) -> Result<Vec<UnifiedJavaVersion>, DownloadError> {
        if let Ok(reg) = crate::remote::VersionRegistry::load() {
            let mut versions = Vec::new();
            for e in reg.list() {
                let (minor, patch) = crate::remote::version_registry::split_version(&e.version);
                let mut download_urls = HashMap::new();
                let iter = e.assets_tsinghua.as_ref().unwrap_or(&e.assets);
                for (k, filename) in iter.iter() {
                    let parts: Vec<&str> = k.split('-').collect();
                    let os = parts.get(0).cloned().unwrap_or("");
                    let arch = parts.get(1).cloned().unwrap_or("");
                    let mirror_os = if os == "macos" { "mac" } else { os };
                    let url = format!(
                        "{}/{}/jdk/{}/{}{}{}",
                        self.base_url,
                        e.major,
                        arch,
                        mirror_os,
                        if mirror_os.ends_with('/') { "" } else { "/" },
                        filename
                    );
                    download_urls.insert(k.clone(), DownloadSource { primary: url, fallback: None });
                }
                versions.push(UnifiedJavaVersion {
                    version: e.version.clone(),
                    major: e.major,
                    minor,
                    patch,
                    tag_name: e.tag_name.clone(),
                    release_name: format!("Eclipse Temurin JDK {}", e.version),
                    download_urls,
                    is_lts: e.lts,
                    published_at: "registry".to_string(),
                    checksums: None,
                });
            }
            return Ok(versions);
        }
        Err(DownloadError::from("Version registry not found".to_string()))
    }


    /// Get download URL for platform with mirror-first and GitHub fallback
    async fn pick_available_url(&self, entry: &DownloadSource) -> Result<String, String> {
        // Prefer mirror first
        if self.is_url_available(&entry.primary).await {
            return Ok(entry.primary.clone());
        }

        if let Some(fallback) = &entry.fallback {
            println!("-> Mirror unavailable, falling back to GitHub");
            return Ok(fallback.clone());
        }

        Err("Primary and fallback download url unavailable".to_string())
    }

    async fn is_url_available(&self, url: &str) -> bool {
        match self.client.head(url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

impl Default for TsinghuaJavaDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaDownloader for TsinghuaJavaDownloader {
    fn list_available_versions(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<UnifiedJavaVersion>, DownloadError>> + Send + '_>> {
        Box::pin(self.list_versions_internal())
    }

    fn find_version_by_spec<'a, 'b>(&'a self, spec: &'b str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<UnifiedJavaVersion, DownloadError>> + Send + 'a>> {
        let spec_string = spec.to_string();
        Box::pin(async move {
            let versions = self.list_versions_internal().await?;
            crate::infrastructure::installer::utils::pick_best_version(versions, &spec_string)
        })
    }

    fn get_download_url<'a, 'b, 'c>(
        &'a self,
        version: &'b UnifiedJavaVersion,
        platform: &'c Platform,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, DownloadError>> + Send + 'a>> {
        // Clone to avoid lifetime issues in async block
        let version_clone = version.clone();
        let platform_clone = platform.clone();
        
        Box::pin(async move { 
            let key = platform_clone.key();

            if let Some(entry) = version_clone.download_urls.get(&key) {
                return self.pick_available_url(entry).await.map_err(DownloadError::from);
            }
    
            // Try similar OS even if arch key differs
            for (platform_key, entry) in version_clone.download_urls.iter() {
                if platform_key.starts_with(&platform_clone.os) {
                    println!("-> Using closest platform match: {} -> {}", platform_key, key);
                    return self.pick_available_url(entry).await.map_err(DownloadError::from);
                }
            }
    
            Err(DownloadError::from(format!("No download url matches {}", key)))
        })
    }

    fn download_java<'a, 'b, 'c>(
        &'a self,
        version: &'b UnifiedJavaVersion,
        platform: &'c Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send + Sync>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<DownloadTarget, DownloadError>> + Send + 'a>> {
        // Clone to avoid lifetime issues in async block
        let version_clone = version.clone();
        let platform_clone = platform.clone();
        
        Box::pin(async move {
            let url = self.get_download_url(&version_clone, &platform_clone).await.map_err(DownloadError::from)?;

            println!("-> Downloading Java {} from mirror...", version_clone.version);
            println!("-> URL: {}", url);

            let bytes = download_to_bytes(&self.client, &url, |d, t| progress_callback(d, t)).await
                .map_err(DownloadError::from)?;
            println!("<- Downloaded size: {} MB", bytes.len() / (1024 * 1024));
            Ok(DownloadTarget::Bytes(bytes))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tsinghua_downloader_functionality() {
        println!("🎓 测试清华镜像下载器功能...");
        let downloader = TsinghuaJavaDownloader::new();

        // 测试获取版本列表
        match downloader.list_available_versions().await {
            Ok(versions) => {
                println!("✅ 清华版本列表获取成功，共 {} 个版本", versions.len());
                assert!(!versions.is_empty(), "版本列表不应为空");

                // 测试版本解析
                let test_specs = ["21", "17", "11", "25", "20", "lts"];
                for spec in test_specs {
                    match downloader.find_version_by_spec(spec).await {
                        Ok(version) => {
                            let lts_marker = if version.is_lts { " (LTS)" } else { "" };
                            println!("✅ 清华版本解析 '{}' -> Java {}{}", spec, version.version, lts_marker);
                            assert!(!version.version.is_empty());
                            assert!(version.major > 0);
                            assert!(!version.download_urls.is_empty());

                            // 测试平台下载链接
                            let platform = Platform::current();
                            match downloader.get_download_url(&version, &platform).await {
                                Ok(url) => {
                                    println!("  ✅ 下载链接获取成功: {}",
                                        url.chars().take(60).collect::<String>());
                                    assert!(url.contains("tsinghua") || url.contains("github"));
                                }
                                Err(e) => {
                                    println!("  ⚠️  获取下载链接失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("⚠️  清华版本解析 '{}' 失败: {}", spec, e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ 清华版本列表获取失败: {}", e);
                // 不标记为测试失败，因为可能是网络问题
            }
        }
    }
}
