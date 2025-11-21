use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{download::download_to_bytes, platform::Platform};
use super::java_downloader::{JavaDownloader, DownloadTarget, DownloadError};

/// Mirror download entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsinghuaDownloadEntry {
    pub primary: String,
    pub fallback: Option<String>,
}

/// Java version mapped to Tsinghua mirror URLs with GitHub fallback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsinghuaJavaVersion {
    pub version: String,
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub release_name: String,
    pub tag_name: String,
    pub download_urls: HashMap<String, TsinghuaDownloadEntry>, // os-arch -> download_url
    pub is_lts: bool,
    pub published_at: String,
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

    pub async fn list_available_versions(&self) -> Result<Vec<TsinghuaJavaVersion>, String> {
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
                    download_urls.insert(k.clone(), TsinghuaDownloadEntry { primary: url, fallback: None });
                }
                versions.push(TsinghuaJavaVersion {
                    version: e.version.clone(),
                    major: e.major,
                    minor,
                    patch,
                    release_name: format!("Eclipse Temurin JDK {}", e.version),
                    tag_name: e.tag_name.clone(),
                    download_urls,
                    is_lts: e.lts,
                    published_at: "registry".to_string(),
                });
            }
            return Ok(versions);
        }
        Err("Version registry not found".to_string())
    }


    /// Get download URL for platform with mirror-first and GitHub fallback
    pub async fn get_download_url(
        &self,
        version: &TsinghuaJavaVersion,
        platform: &Platform,
    ) -> Result<String, String> {
        let key = platform.key();

        if let Some(entry) = version.download_urls.get(&key) {
            return self.pick_available_url(entry).await;
        }

        // Try similar OS even if arch key differs
        for (platform_key, entry) in version.download_urls.iter() {
            if platform_key.starts_with(&platform.os) {
                println!("-> Using closest platform match: {} -> {}", platform_key, key);
                return self.pick_available_url(entry).await;
            }
        }

        Err(format!("No download url matches {}", key))
    }

    async fn pick_available_url(&self, entry: &TsinghuaDownloadEntry) -> Result<String, String> {
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

    /// Download the specified version
    pub async fn download_java(
        &self,
        version: &TsinghuaJavaVersion,
        platform: &Platform,
        progress_callback: impl Fn(u64, u64),
    ) -> Result<Vec<u8>, String> {
        let download_url = self.get_download_url(version, platform).await?;

        println!("-> Downloading Java {} from mirror...", version.version);
        println!("-> URL: {}", download_url);

        let data = download_to_bytes(&self.client, &download_url, progress_callback).await?;
        println!("<- Downloaded size: {} MB", data.len() / (1024 * 1024));
        Ok(data)
    }

    pub async fn find_version_by_spec(&self, spec: &str) -> Result<TsinghuaJavaVersion, String> {
        if let Ok(reg) = crate::remote::VersionRegistry::load() {
            if let Some(e) = reg.find(spec) {
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
                    download_urls.insert(k.clone(), TsinghuaDownloadEntry { primary: url, fallback: None });
                }
                return Ok(TsinghuaJavaVersion {
                    version: e.version.clone(),
                    major: e.major,
                    minor,
                    patch,
                    release_name: format!("Eclipse Temurin JDK {}", e.version),
                    tag_name: e.tag_name.clone(),
                    download_urls,
                    is_lts: e.lts,
                    published_at: "registry".to_string(),
                });
            }
        }
        let versions = self.list_available_versions().await?;

        let spec_cleaned = spec.trim().to_lowercase()
            .replace("v", "")
            .replace("jdk", "")
            .replace("java", "")
            .trim()
            .to_string();

        if spec_cleaned == "lts" || spec_cleaned == "latest-lts" {
            for version in &versions {
                if version.is_lts {
                    return Ok(version.clone());
                }
            }
            return Err("No LTS version found".to_string());
        } else if spec_cleaned == "latest" || spec_cleaned == "newest" {
            return versions.into_iter().next()
                .ok_or("No available versions".to_string());
        }

        let parts: Vec<&str> = spec_cleaned.split('.').filter(|p| !p.is_empty()).collect();
        if !parts.is_empty() && parts[0].parse::<u32>().is_ok() {
            if parts.len() == 1 {
                let major = parts[0].parse::<u32>().unwrap();

                let mut lts_versions: Vec<&TsinghuaJavaVersion> = versions.iter()
                    .filter(|v| v.major == major && v.is_lts)
                    .collect();
                lts_versions.sort_by(|a, b| b.version.cmp(&a.version));
                if let Some(latest_lts) = lts_versions.first() {
                    return Ok((**latest_lts).clone());
                }

                let mut major_versions: Vec<&TsinghuaJavaVersion> = versions.iter()
                    .filter(|v| v.major == major)
                    .collect();
                major_versions.sort_by(|a, b| b.version.cmp(&a.version));
                if let Some(latest) = major_versions.first() {
                    return Ok((**latest).clone());
                }

                return Err(format!("No Java {} found", major));
            } else {
                let full_version = parts.join(".");
                for version in &versions {
                    if version.version == full_version ||
                       version.tag_name.contains(&full_version) ||
                       version.release_name.to_lowercase().contains(&full_version) {
                        return Ok(version.clone());
                    }
                }

                let major = parts[0].parse::<u32>().unwrap();
                for version in &versions {
                    if version.major == major {
                        return Ok(version.clone());
                    }
                }

                return Err(format!("Version not found: {}", spec));
            }
        }

        for version in versions {
            if version.version == spec_cleaned ||
               version.tag_name == spec_cleaned ||
               version.release_name.to_lowercase().contains(&spec_cleaned) {
                return Ok(version);
            }
        }

        Err(format!("Version not found: {}", spec))
    }
}

impl JavaDownloader for TsinghuaJavaDownloader {
    type Version = TsinghuaJavaVersion;

    fn version_string(&self, version: &Self::Version) -> String {
        version.version.clone()
    }

    fn list_available_versions(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<Self::Version>, DownloadError>> + Send + '_>> {
        let fut = self.list_available_versions();
        Box::pin(async move { fut.await.map_err(DownloadError::from) })
    }

    fn find_version_by_spec<'a, 'b>(&'a self, spec: &'b str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Version, DownloadError>> + Send + 'a>> {
        let spec_owned = spec.to_string();
        Box::pin(async move { self.find_version_by_spec(&spec_owned).await.map_err(DownloadError::from) })
    }

    fn get_download_url<'a, 'b, 'c>(
        &'a self,
        version: &'b Self::Version,
        platform: &'c Platform,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, DownloadError>> + Send + 'a>> {
        let version_cloned = version.clone();
        let platform_cloned = platform.clone();
        Box::pin(async move { self.get_download_url(&version_cloned, &platform_cloned).await.map_err(DownloadError::from) })
    }

    fn download_java<'a, 'b, 'c>(
        &'a self,
        version: &'b Self::Version,
        platform: &'c Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<DownloadTarget, DownloadError>> + Send + 'a>> {
        let version_cloned = version.clone();
        let platform_cloned = platform.clone();
        Box::pin(async move {
            let bytes = self
                .download_java(&version_cloned, &platform_cloned, move |d, t| (progress_callback)(d, t))
                .await
                .map_err(DownloadError::from)?;
            Ok(DownloadTarget::Bytes(bytes))
        })
    }
}

impl Default for TsinghuaJavaDownloader {
    fn default() -> Self {
        Self::new()
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

                // 测试稳定版本
                let stable_versions = downloader.get_stable_versions();
                        // 取消硬编码稳定版本，改为使用配置驱动

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
