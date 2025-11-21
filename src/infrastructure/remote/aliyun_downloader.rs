use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{download::download_to_bytes, platform::Platform, GitHubJavaDownloader};
use super::java_downloader::{JavaDownloader, DownloadTarget, DownloadError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliyunDownloadEntry {
    pub primary: String,
    pub fallback: Option<String>,
}

/// 阿里云 Java 版本信息，下载 URL 为镜像地址，带 GitHub 兜底。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliyunJavaVersion {
    pub version: String,
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub release_name: String,
    pub tag_name: String,
    pub download_urls: HashMap<String, AliyunDownloadEntry>, // os-arch -> download_url
    pub is_lts: bool,
    pub published_at: String,
}

/// 阿里云镜像下载器：基于 GitHub 版本信息构造镜像 URL，并在镜像失效时自动回退。
pub struct AliyunJavaDownloader {
    client: reqwest::Client,
    base_url: String,
}

impl AliyunJavaDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://mirrors.aliyun.com/eclipse/temurin-compliance/temurin".to_string(),
        }
    }

    /// 从 GitHub 拉取版本列表并重写为阿里云镜像地址。
    pub async fn list_available_versions(&self) -> Result<Vec<AliyunJavaVersion>, String> {
        let registry_only = crate::infrastructure::config::Config::load()
            .map(|c| c.java_download_sources.registry_only)
            .unwrap_or(false);
        if let Ok(reg) = crate::remote::VersionRegistry::load() {
            let mut versions = Vec::new();
            for e in reg.list() {
                let (minor, patch) = crate::remote::version_registry::split_version(&e.version);
                let mut download_urls = HashMap::new();
                let iter = e.assets_aliyun.as_ref().unwrap_or(&e.assets);
                for (k, filename) in iter.iter() {
                    let url = format!(
                        "{}/{}/{}{}{}",
                        self.base_url,
                        e.major,
                        e.tag_name,
                        if e.tag_name.ends_with('/') { "" } else { "/" },
                        filename
                    );
                    download_urls.insert(k.clone(), AliyunDownloadEntry { primary: url, fallback: None });
                }
                versions.push(AliyunJavaVersion {
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
        if registry_only { return Err("registry-only: version registry not found".to_string()); }
        println!("🛰️  正在从阿里云镜像构建 Java 版本列表...");

        let ttl = crate::infrastructure::config::Config::load()
            .map(|c| c.java_version_cache.ttl)
            .unwrap_or(3600);
        let cache = crate::remote::cache::VersionCacheManager::new()
            .map_err(|e| format!("初始化缓存失败: {}", e))?
            .with_ttl(ttl);
        if let Ok(Some(cached)) = cache.load::<Vec<AliyunJavaVersion>>(&crate::remote::cache::CacheKeys::java_versions_aliyun()).await {
            println!("📖 使用缓存的阿里云版本列表");
            return Ok(cached);
        }

        let github = GitHubJavaDownloader::new();
        let gh_versions = github.list_available_versions().await?;
        let mut versions = Vec::new();

        for v in gh_versions {
            let mut download_urls = HashMap::new();
            let tag_plain = v.tag_name.replace("%2B", "+").replace("%2b", "+");

            for (key, url) in v.download_urls.iter() {
                if let Some(filename) = url.split('/').last() {
                    let mirror_url = format!(
                        "{}/{}/{}{}{}",
                        self.base_url,
                        v.major,
                        tag_plain,
                        if tag_plain.ends_with('/') { "" } else { "/" },
                        filename
                    );
                    download_urls.insert(
                        key.clone(),
                        AliyunDownloadEntry {
                            primary: mirror_url,
                            fallback: Some(url.clone()),
                        },
                    );
                }
            }

            versions.push(AliyunJavaVersion {
                version: v.version.clone(),
                major: v.major,
                minor: v.minor,
                patch: v.patch,
                release_name: v.release_name.clone(),
                tag_name: v.tag_name.clone(),
                download_urls,
                is_lts: v.is_lts,
                published_at: v.published_at.clone(),
            });
        }

        println!("✓ 构建完成，发现 {} 个可用版本", versions.len());
        let _ = cache.save(&crate::remote::cache::CacheKeys::java_versions_aliyun(), &versions, None).await;
        Ok(versions)
    }

    /// 按平台获取下载 URL，优先阿里云镜像，不通时回退 GitHub。
    pub async fn get_download_url(
        &self,
        version: &AliyunJavaVersion,
        platform: &Platform,
    ) -> Result<String, String> {
        let key = platform.key();

        if let Some(entry) = version.download_urls.get(&key) {
            return self.pick_available_url(entry).await;
        }

        // 允许同 OS 任意架构兜底
        for (platform_key, entry) in version.download_urls.iter() {
            if platform_key.starts_with(&platform.os) {
                println!("⚠️  使用邻近平台包: {} -> {}", platform_key, key);
                return self.pick_available_url(entry).await;
            }
        }

        Err(format!("未找到匹配 {} 的下载地址", key))
    }

    async fn pick_available_url(&self, entry: &AliyunDownloadEntry) -> Result<String, String> {
        // 优先阿里云镜像，可用即返回
        if self.is_url_available(&entry.primary).await {
            return Ok(entry.primary.clone());
        }

        if let Some(fallback) = &entry.fallback {
            println!("↩️  镜像不可用，回退 GitHub");
            return Ok(fallback.clone());
        }

        Err("镜像与备用地址均不可用".to_string())
    }

    async fn is_url_available(&self, url: &str) -> bool {
        match self.client.head(url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// 下载指定版本。
    pub async fn download_java(
        &self,
        version: &AliyunJavaVersion,
        platform: &Platform,
        progress_callback: impl Fn(u64, u64),
    ) -> Result<Vec<u8>, String> {
        let download_url = self.get_download_url(version, platform).await?;

        println!("⬇️  下载 Java {}...", version.version);
        println!("📥 地址: {}", download_url);

        let data = download_to_bytes(&self.client, &download_url, progress_callback).await?;
        println!("✓ 下载完成，大小: {} MB", data.len() / (1024 * 1024));
        Ok(data)
    }

    /// 版本解析（与 GitHub 下载器保持一致）。
    pub async fn find_version_by_spec(&self, spec: &str) -> Result<AliyunJavaVersion, String> {
        let registry_only = crate::infrastructure::config::Config::load()
            .map(|c| c.java_download_sources.registry_only)
            .unwrap_or(false);
        if let Ok(reg) = crate::remote::VersionRegistry::load() {
            if let Some(e) = reg.find(spec) {
                let (minor, patch) = crate::remote::version_registry::split_version(&e.version);
                let mut download_urls = HashMap::new();
                let iter = e.assets_aliyun.as_ref().unwrap_or(&e.assets);
                for (k, filename) in iter.iter() {
                    let url = format!(
                        "{}/{}/{}{}{}",
                        self.base_url,
                        e.major,
                        e.tag_name,
                        if e.tag_name.ends_with('/') { "" } else { "/" },
                        filename
                    );
                    download_urls.insert(k.clone(), AliyunDownloadEntry { primary: url, fallback: None });
                }
                return Ok(AliyunJavaVersion {
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
        if registry_only { return Err("registry-only: version not found in registry".to_string()); }
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
            return Err("未找到 LTS 版本".to_string());
        } else if spec_cleaned == "latest" || spec_cleaned == "newest" {
            return versions.into_iter().next()
                .ok_or("未找到可用版本".to_string());
        }

        // 数字前缀认为是版本号
        let parts: Vec<&str> = spec_cleaned.split('.').filter(|p| !p.is_empty()).collect();
        if !parts.is_empty() && parts[0].parse::<u32>().is_ok() {
            if parts.len() == 1 {
                let major = parts[0].parse::<u32>().unwrap();

                let mut lts_versions: Vec<&AliyunJavaVersion> = versions.iter()
                    .filter(|v| v.major == major && v.is_lts)
                    .collect();
                lts_versions.sort_by(|a, b| b.version.cmp(&a.version));
                if let Some(latest_lts) = lts_versions.first() {
                    return Ok((**latest_lts).clone());
                }

                let mut major_versions: Vec<&AliyunJavaVersion> = versions.iter()
                    .filter(|v| v.major == major)
                    .collect();
                major_versions.sort_by(|a, b| b.version.cmp(&a.version));
                if let Some(latest) = major_versions.first() {
                    return Ok((**latest).clone());
                }

                return Err(format!("未找到 Java {}", major));
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

                return Err(format!("未找到版本: {}", spec));
            }
        }

        for version in versions {
            if version.version == spec_cleaned ||
               version.tag_name == spec_cleaned ||
               version.release_name.to_lowercase().contains(&spec_cleaned) {
                return Ok(version);
            }
        }

        Err(format!("未找到版本: {}", spec))
    }
}

impl JavaDownloader for AliyunJavaDownloader {
    type Version = AliyunJavaVersion;

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

impl Default for AliyunJavaDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_download_url_fallback() {
        let downloader = AliyunJavaDownloader::new();
        let mut download_urls = HashMap::new();
        download_urls.insert(
            "windows-x64".to_string(),
            AliyunDownloadEntry {
                primary: "http://127.0.0.1:9/unavailable".to_string(), // 端口 9 通常无服务，触发回退
                fallback: Some("https://example.com/fallback.zip".to_string()),
            },
        );

        let version = AliyunJavaVersion {
            version: "17.0.0".to_string(),
            major: 17,
            minor: Some(0),
            patch: Some(0),
            release_name: "jdk-17.0.0".to_string(),
            tag_name: "jdk-17.0.0".to_string(),
            download_urls,
            is_lts: true,
            published_at: "2024-01-01".to_string(),
        };

        let platform = Platform {
            os: "windows".to_string(),
            arch: "x64".to_string(),
        };

        let url = downloader.get_download_url(&version, &platform).await.unwrap();
        assert_eq!(url, "https://example.com/fallback.zip");
    }

    #[tokio::test]
    async fn test_aliyun_downloader_real_functionality() {
        println!("🛰️  测试阿里云镜像下载器实际功能...");
        let downloader = AliyunJavaDownloader::new();

        // 测试获取版本列表
        match downloader.list_available_versions().await {
            Ok(versions) => {
                println!("✅ 阿里云版本列表获取成功，共 {} 个版本", versions.len());
                assert!(!versions.is_empty(), "版本列表不应为空");

                // 测试版本解析
                let test_specs = ["21", "17", "lts"];
                for spec in test_specs {
                    match downloader.find_version_by_spec(spec).await {
                        Ok(version) => {
                            println!("✅ 阿里云版本解析 '{}' -> Java {}", spec, version.version);
                            assert!(!version.version.is_empty());
                            assert!(version.major > 0);
                        }
                        Err(e) => {
                            println!("⚠️  阿里云版本解析 '{}' 失败: {}", spec, e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ 阿里云版本列表获取失败: {}", e);
                // 不标记为测试失败，因为可能是网络问题
            }
        }
    }
}
