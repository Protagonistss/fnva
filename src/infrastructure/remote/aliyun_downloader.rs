use reqwest;
use std::collections::HashMap;

use super::java_downloader::{DownloadError, DownloadTarget, JavaDownloader};
use super::mirror_utils;
use super::DownloadSource;
use super::GitHubJavaDownloader;
use super::UnifiedJavaVersion;
use super::{download::download_to_file, platform::Platform};

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
    async fn list_versions_internal(&self) -> Result<Vec<UnifiedJavaVersion>, DownloadError> {
        let registry_only = crate::infrastructure::config::Config::load()
            .map(|c| c.java_download_sources.registry_only)
            .unwrap_or(false);
        if let Ok(reg) = crate::remote::VersionRegistry::load() {
            let mut versions = Vec::new();
            for e in reg.list() {
                let (minor, patch) = crate::remote::version_registry::split_version(&e.version);
                let mut download_urls = HashMap::new();
                let iter = &e.assets;
                for (k, filename) in iter.iter() {
                    let url = format!(
                        "{}/{}/{}{}{}",
                        self.base_url,
                        e.major,
                        e.tag_name,
                        if e.tag_name.ends_with('/') { "" } else { "/" },
                        filename
                    );
                    download_urls.insert(
                        k.clone(),
                        DownloadSource {
                            primary: url,
                            fallback: None,
                        },
                    );
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
        if registry_only {
            return Err(DownloadError::from(
                "registry-only: version registry not found".to_string(),
            ));
        }
        println!("🛰️  正在从阿里云镜像构建 Java 版本列表...");

        let ttl = crate::infrastructure::config::Config::load()
            .map(|c| c.java_version_cache.ttl)
            .unwrap_or(3600);
        let cache = crate::remote::cache::VersionCacheManager::new()
            .map_err(|e| DownloadError::from(format!("初始化缓存失败: {e}")))?
            .with_ttl(ttl);
        if let Ok(Some(cached)) = cache
            .load::<Vec<UnifiedJavaVersion>>(
                &crate::remote::cache::CacheKeys::java_versions_aliyun(),
            )
            .await
        {
            println!("📖 使用缓存的阿里云版本列表");
            return Ok(cached);
        }

        let github = GitHubJavaDownloader::new();
        // Call list_available_versions via trait to get UnifiedJavaVersion
        let gh_versions = github.list_available_versions().await?;
        let mut versions = Vec::new();

        for v in gh_versions {
            let mut download_urls = HashMap::new();
            let tag_plain = v.tag_name.replace("%2B", "+").replace("%2b", "+");

            for (key, source) in v.download_urls.iter() {
                let url = &source.primary;
                if let Some(filename) = url.split('/').next_back() {
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
                        DownloadSource {
                            primary: mirror_url,
                            fallback: Some(url.clone()),
                        },
                    );
                }
            }

            versions.push(UnifiedJavaVersion {
                version: v.version.clone(),
                major: v.major,
                minor: v.minor,
                patch: v.patch,
                release_name: v.release_name.clone(),
                tag_name: v.tag_name.clone(),
                download_urls,
                is_lts: v.is_lts,
                published_at: v.published_at.clone(),
                checksums: None,
            });
        }

        println!("✓ 构建完成，发现 {} 个可用版本", versions.len());
        let _ = cache
            .save(
                &crate::remote::cache::CacheKeys::java_versions_aliyun(),
                &versions,
                None,
            )
            .await;
        Ok(versions)
    }
}

impl Default for AliyunJavaDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaDownloader for AliyunJavaDownloader {
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

            if let Some(entry) = version_clone.download_urls.get(&key) {
                match mirror_utils::pick_available_url(&self.client, entry).await {
                    Ok(url) => {
                        if url != entry.primary {
                            println!("↩️  镜像不可用，回退 GitHub");
                        }
                        return Ok(url);
                    }
                    Err(e) => return Err(DownloadError::from(e)),
                }
            }

            // 允许同 OS 任意架构兜底
            for (platform_key, entry) in version_clone.download_urls.iter() {
                if platform_key.starts_with(&platform_clone.os) {
                    println!("⚠️  使用邻近平台包: {platform_key} -> {key}");
                    match mirror_utils::pick_available_url(&self.client, entry).await {
                        Ok(url) => {
                            if url != entry.primary {
                                println!("↩️  镜像不可用，回退 GitHub");
                            }
                            return Ok(url);
                        }
                        Err(e) => return Err(DownloadError::from(e)),
                    }
                }
            }

            Err(DownloadError::from(format!("未找到匹配 {key} 的下载地址")))
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

            println!("⬇️  下载 Java {}...", version_clone.version);
            println!("📥 地址: {url}");

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
                "OpenJDK-{}-{}.{}-aliyun.{}",
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

            download_to_file(&self.client, &url, &file_path, |d, t| {
                progress_callback(d, t)
            })
            .await
            .map_err(|e| DownloadError::from(format!("下载失败: {e}")))?;

            let file_size = tokio::fs::metadata(&file_path)
                .await
                .map_err(|e| DownloadError::Io(format!("获取文件大小失败: {e}")))?
                .len();
            println!("✓ 下载完成，大小: {} MB", file_size / (1024 * 1024));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_download_url_fallback() {
        let downloader = AliyunJavaDownloader::new();
        let mut download_urls = HashMap::new();
        download_urls.insert(
            "windows-x64".to_string(),
            DownloadSource {
                primary: "http://127.0.0.1:9/unavailable".to_string(), // 端口 9 通常无服务，触发回退
                fallback: Some("https://example.com/fallback.zip".to_string()),
            },
        );

        let version = UnifiedJavaVersion {
            version: "17.0.0".to_string(),
            major: 17,
            minor: Some(0),
            patch: Some(0),
            release_name: "jdk-17.0.0".to_string(),
            tag_name: "jdk-17.0.0".to_string(),
            download_urls,
            is_lts: true,
            published_at: "2024-01-01".to_string(),
            checksums: None,
        };

        let platform = Platform {
            os: "windows".to_string(),
            arch: "x64".to_string(),
        };

        let url = downloader
            .get_download_url(&version, &platform)
            .await
            .unwrap();
        assert_eq!(url, "https://example.com/fallback.zip");
    }
}
