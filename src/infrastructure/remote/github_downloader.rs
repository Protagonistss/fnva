use super::download::download_to_file;
use super::java_downloader::{DownloadError, DownloadTarget, JavaDownloader};
use super::platform::Platform;
use super::DownloadSource;
use super::UnifiedJavaVersion;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GitHub Java 发行版信息（从 jdk 仓库获取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubJavaRelease {
    pub tag_name: String,
    pub name: String,
    pub prerelease: bool,
    pub published_at: String,
    pub assets: Vec<GitHubAsset>,
    pub html_url: String,
}

/// GitHub 资源文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: String,
}

/// GitHub Java 下载器
pub struct GitHubJavaDownloader {
    client: reqwest::Client,
    api_base_url: String,
}

impl GitHubJavaDownloader {
    /// 创建新的 GitHub Java 下载器
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base_url: "https://api.github.com".to_string(),
        }
    }

    /// 从 GitHub 发行版解析版本信息
    fn parse_version_from_release(
        &self,
        release: &GitHubJavaRelease,
    ) -> Result<UnifiedJavaVersion, String> {
        let tag_name = &release.tag_name;

        // adoptium/jdk 的标签格式可能是：jdk-17.0.8+7, jdk-11.0.23+9 等
        let version_part = if let Some(version) = tag_name.strip_prefix("jdk-") {
            version
        } else {
            return Err(format!("无效的标签格式: {tag_name}"));
        };

        // 移除构建号部分，如 "17.0.8+7" -> "17.0.8"
        let clean_version = version_part.split('+').next().unwrap_or(version_part);

        let version_parts: Vec<&str> = clean_version.split('.').collect();
        if version_parts.len() < 2 {
            return Err("版本格式无效".to_string());
        }

        let major = version_parts[0]
            .parse::<u32>()
            .map_err(|_| "无效的主版本号")?;
        let minor = version_parts.get(1).and_then(|s| s.parse::<u32>().ok());
        let patch = version_parts.get(2).and_then(|s| s.parse::<u32>().ok());

        // 判断是否为 LTS 版本
        let is_lts = [8, 11, 17, 21, 25].contains(&major);

        // 解析下载链接
        let mut download_urls = HashMap::new();

        for asset in &release.assets {
            if let Some((os, arch)) = Platform::parse_from_filename(&asset.name) {
                download_urls.insert(
                    format!("{os}-{arch}"),
                    DownloadSource {
                        primary: asset.browser_download_url.clone(),
                        fallback: None,
                    },
                );
            }
        }

        Ok(UnifiedJavaVersion {
            version: clean_version.to_string(),
            major,
            minor,
            patch,
            release_name: release.name.clone(),
            tag_name: tag_name.clone(),
            download_urls,
            is_lts,
            published_at: release.published_at.clone(),
            checksums: None, // GitHub API 不直接返回 checksum，后续可以增强
        })
    }

    async fn list_versions_internal(&self) -> Result<Vec<UnifiedJavaVersion>, DownloadError> {
        let registry_only = crate::infrastructure::config::Config::load()
            .map(|c| c.java_download_sources.registry_only)
            .unwrap_or(false);
        if let Ok(reg) = crate::remote::VersionRegistry::load() {
            let mut result = Vec::new();
            for e in reg.list() {
                let (minor, patch) = crate::remote::version_registry::split_version(&e.version);
                let mut download_urls = HashMap::new();
                let iter = &e.assets;
                for (k, filename) in iter.iter() {
                    let url = format!(
                        "https://github.com/adoptium/temurin{}-binaries/releases/download/{}/{}",
                        e.major, e.tag_name, filename
                    );
                    download_urls.insert(
                        k.clone(),
                        DownloadSource {
                            primary: url,
                            fallback: None,
                        },
                    );
                }
                result.push(UnifiedJavaVersion {
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
            return Ok(result);
        }
        if registry_only {
            return Err(DownloadError::from(
                "registry-only: version registry not found".to_string(),
            ));
        }
        println!("🔍 正在从 GitHub 查询可用的 Java 版本...");

        let ttl = crate::infrastructure::config::Config::load()
            .map(|c| c.java_version_cache.ttl)
            .unwrap_or(3600);
        let cache = crate::remote::cache::VersionCacheManager::new()
            .map_err(|e| DownloadError::from(format!("初始化缓存失败: {e}")))?
            .with_ttl(ttl);
        if let Ok(Some(cached)) = cache
            .load::<Vec<UnifiedJavaVersion>>(
                &crate::remote::cache::CacheKeys::java_versions_github(),
            )
            .await
        {
            println!("📖 使用缓存的 GitHub 版本列表");
            return Ok(cached);
        }

        // 尝试多个 Adoptium GitHub 仓库
        let repositories = vec![
            "adoptium/temurin25-binaries",
            "adoptium/temurin21-binaries",
            "adoptium/temurin17-binaries",
            "adoptium/temurin11-binaries",
            "adoptium/temurin8-binaries",
        ];

        let mut all_versions = Vec::new();
        let mut seen_versions = std::collections::HashSet::new();

        for repo in repositories {
            println!("📦 检查仓库: {repo}");

            let url = format!("{}/repos/{}/releases", self.api_base_url, repo);

            let response = self
                .client
                .get(&url)
                .header("User-Agent", "fnva/0.0.5")
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .await
                .map_err(|e| DownloadError::from(format!("请求 GitHub API 失败: {e}")))?;

            if !response.status().is_success() {
                println!("⚠️  仓库 {} 访问失败: {}", repo, response.status());
                continue;
            }

            let releases: Vec<GitHubJavaRelease> = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    println!("⚠️  解析仓库 {repo} 响应失败: {e}");
                    continue;
                }
            };

            for release in releases.into_iter().take(5) {
                // 每个仓库最多取5个版本
                // 跳过预发布版本
                if release.prerelease {
                    continue;
                }

                // 解析版本信息
                if let Ok(version_info) = self.parse_version_from_release(&release) {
                    // 避免重复版本
                    let version_key = format!(
                        "{}.{}.{}",
                        version_info.major,
                        version_info.minor.unwrap_or(0),
                        version_info.patch.unwrap_or(0)
                    );

                    if !seen_versions.contains(&version_key) {
                        seen_versions.insert(version_key);
                        all_versions.push(version_info);
                    }
                }
            }
        }

        // 按版本号排序
        all_versions.sort_by(|a, b| {
            b.major
                .cmp(&a.major)
                .then(b.minor.cmp(&a.minor))
                .then(b.patch.cmp(&a.patch))
        });

        println!("✅ 找到 {} 个可用版本", all_versions.len());
        let _ = cache
            .save(
                &crate::remote::cache::CacheKeys::java_versions_github(),
                &all_versions,
                None,
            )
            .await;
        Ok(all_versions)
    }
}

impl Default for GitHubJavaDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaDownloader for GitHubJavaDownloader {
    fn list_available_versions(
        &self,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<UnifiedJavaVersion>, DownloadError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(self.list_versions_internal())
    }

    fn find_version_by_spec<'a, 'b>(
        &'a self,
        spec: &'b str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<UnifiedJavaVersion, DownloadError>> + Send + 'a,
        >,
    > {
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
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, DownloadError>> + Send + 'a>,
    > {
        // Clone to avoid lifetime issues in async block
        let version_clone = version.clone();
        let platform_clone = platform.clone();

        Box::pin(async move {
            let key = platform_clone.key();
            if let Some(source) = version_clone.download_urls.get(&key) {
                return Ok(source.primary.clone());
            }
            // 尝试匹配相似的配置
            for (platform_key, source) in &version_clone.download_urls {
                if platform_key.starts_with(&platform_clone.os) {
                    println!("⚠️  使用相似的架构: {platform_key} -> {key}");
                    return Ok(source.primary.clone());
                }
            }
            Err(DownloadError::from(format!(
                "未找到适合 {}-{} 的下载链接",
                platform_clone.os, platform_clone.arch
            )))
        })
    }

    fn download_java<'a, 'b, 'c>(
        &'a self,
        version: &'b UnifiedJavaVersion,
        platform: &'c Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send + Sync>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<DownloadTarget, DownloadError>> + Send + 'a>,
    > {
        // Clone to avoid lifetime issues in async block
        let version_clone = version.clone();
        let platform_clone = platform.clone();

        Box::pin(async move {
            let url = self
                .get_download_url(&version_clone, &platform_clone)
                .await?;

            println!("📥 正在下载 Java {}...", version_clone.version);
            println!("🔗 下载地址: {url}");

            // 创建持久化文件路径而不是临时目录
            let cache_dir = dirs::home_dir()
                .ok_or_else(|| DownloadError::Io("无法获取用户主目录".to_string()))?
                .join(".fnva")
                .join("cache")
                .join("downloads");

            // 确保缓存目录存在
            tokio::fs::create_dir_all(&cache_dir)
                .await
                .map_err(|e| DownloadError::Io(format!("创建缓存目录失败: {e}")))?;

            let extension = platform_clone.archive_ext();
            let file_name = format!(
                "OpenJDK-{}-{}.{}-github.{}",
                version_clone.version, platform_clone.os, platform_clone.arch, extension
            );
            let file_path = cache_dir.join(&file_name);

            // 如果文件已存在且大小正确，跳过下载
            if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                let file_size = metadata.len();
                if file_size > 0 {
                    println!("-> 使用已存在的文件: {} MB", file_size / (1024 * 1024));

                    // 验证文件确实存在
                    if !file_path.exists() {
                        return Err(DownloadError::Io(format!("缓存文件不存在: {file_path:?}")));
                    }

                    // 使用规范化路径，确保在 Windows 上正确处理
                    let canonical_path = file_path
                        .canonicalize()
                        .map_err(|e| DownloadError::Io(format!("无法获取规范路径: {e}")))?;

                    let path_str = canonical_path
                        .to_str()
                        .ok_or_else(|| DownloadError::Io("路径包含无效字符".to_string()))?
                        .to_string();

                    println!("-> 文件保存位置: {path_str}");
                    return Ok(DownloadTarget::File(path_str));
                }
            }

            download_to_file(&self.client, &url, &file_path, |c, t| {
                progress_callback(c, t)
            })
            .await
            .map_err(|e| DownloadError::from(format!("下载失败: {e}")))?;

            let file_size = tokio::fs::metadata(&file_path)
                .await
                .map_err(|e| DownloadError::Io(format!("获取文件大小失败: {e}")))?
                .len();
            println!("✅ 下载完成，大小: {} MB", file_size / (1024 * 1024));

            // 验证文件确实存在
            if !file_path.exists() {
                return Err(DownloadError::Io(format!(
                    "下载的文件不存在: {file_path:?}"
                )));
            }

            // 使用规范化路径，确保在 Windows 上正确处理
            let canonical_path = file_path
                .canonicalize()
                .map_err(|e| DownloadError::Io(format!("无法获取规范路径: {e}")))?;

            let path_str = canonical_path
                .to_str()
                .ok_or_else(|| DownloadError::Io("路径包含无效字符".to_string()))?
                .to_string();

            println!("-> 文件保存位置: {path_str}");

            // 返回持久化文件路径
            Ok(DownloadTarget::File(path_str))
        })
    }
}
