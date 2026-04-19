use super::download::download_to_file;
use super::java_downloader::{DownloadError, DownloadTarget, JavaDownloader};
use super::platform::Platform;
use super::version_registry::{RegistryEntry, VersionRegistry};
use super::UnifiedJavaVersion;
use crate::infrastructure::config::MirrorConfig;
use std::collections::HashMap;

pub struct TemplateDownloader {
    client: reqwest::Client,
    mirrors: Vec<MirrorConfig>,
}

impl TemplateDownloader {
    pub fn new(mirrors: Vec<MirrorConfig>) -> Self {
        let mut sorted = mirrors;
        sorted.sort_by_key(|m| m.priority);
        Self {
            client: reqwest::Client::new(),
            mirrors: sorted,
        }
    }

    fn render_url(template: &str, base_url: &str, major: u32, tag: &str, filename: &str, os: &str, arch: &str) -> String {
        template
            .replace("{base_url}", base_url)
            .replace("{major}", &major.to_string())
            .replace("{tag}", tag)
            .replace("{filename}", filename)
            .replace("{os}", os)
            .replace("{arch}", arch)
    }

    fn entry_to_version(entry: &RegistryEntry) -> UnifiedJavaVersion {
        let (minor, patch) = super::version_registry::split_version(&entry.version);
        UnifiedJavaVersion {
            version: entry.version.clone(),
            major: entry.major,
            minor,
            patch,
            release_name: format!("Eclipse Temurin JDK {}", entry.version),
            tag_name: entry.tag_name.clone(),
            download_urls: HashMap::new(),
            is_lts: entry.lts,
            published_at: "registry".to_string(),
            checksums: None,
        }
    }

    async fn list_versions_internal(&self) -> Result<Vec<UnifiedJavaVersion>, DownloadError> {
        let reg = VersionRegistry::load()
            .map_err(|e| DownloadError::from(format!("Failed to load version registry: {e}")))?;
        Ok(reg.list().iter().map(TemplateDownloader::entry_to_version).collect())
    }
}

impl JavaDownloader for TemplateDownloader {
    fn list_available_versions(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<UnifiedJavaVersion>, DownloadError>> + Send + '_>> {
        Box::pin(self.list_versions_internal())
    }

    fn find_version_by_spec<'a, 'b>(
        &'a self,
        spec: &'b str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<UnifiedJavaVersion, DownloadError>> + Send + 'a>> {
        let spec_string = spec.to_string();
        Box::pin(async move {
            let reg = VersionRegistry::load()
                .map_err(|e| DownloadError::from(format!("Failed to load version registry: {e}")))?;
            let entry = reg.find(&spec_string)
                .ok_or_else(|| DownloadError::from(format!("No version matching '{spec_string}'")))?;
            Ok(Self::entry_to_version(&entry))
        })
    }

    fn get_download_url<'a, 'b, 'c>(
        &'a self,
        version: &'b UnifiedJavaVersion,
        platform: &'c Platform,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, DownloadError>> + Send + 'a>> {
        let mirrors = self.mirrors.clone();
        let version_clone = version.clone();
        let platform_clone = platform.clone();

        Box::pin(async move {
            let reg = VersionRegistry::load()
                .map_err(|e| DownloadError::from(format!("Failed to load version registry: {e}")))?;
            let entry = reg.find(&version_clone.version)
                .ok_or_else(|| DownloadError::from(format!("Version {} not in registry", version_clone.version)))?;

            let platform_key = platform_clone.key();
            let filename = entry.assets.get(&platform_key)
                .or_else(|| entry.assets.keys().find(|k| k.starts_with(&platform_clone.os)).and_then(|k| entry.assets.get(k)))
                .ok_or_else(|| DownloadError::from(format!("No asset found for {platform_key}")))?;

            let parts: Vec<&str> = platform_key.split('-').collect();
            let os = parts.first().cloned().unwrap_or("");
            let arch = parts.get(1).cloned().unwrap_or("");

            for mirror in &mirrors {
                if !mirror.enabled {
                    continue;
                }
                let url = Self::render_url(&mirror.url_template, &mirror.base_url, entry.major, &entry.tag_name, filename, os, arch);
                if super::mirror_utils::is_url_available_with_timeout(&self.client, &url, std::time::Duration::from_secs(5)).await {
                    return Ok(url);
                }
            }

            Err(DownloadError::from("All mirrors unavailable".to_string()))
        })
    }

    fn download_java<'a, 'b, 'c>(
        &'a self,
        version: &'b UnifiedJavaVersion,
        platform: &'c Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send + Sync>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<DownloadTarget, DownloadError>> + Send + 'a>> {
        let mirrors = self.mirrors.clone();
        let version_clone = version.clone();
        let platform_clone = platform.clone();

        Box::pin(async move {
            let url = self.get_download_url(&version_clone, &platform_clone).await?;
            println!("-> Downloading Java {} ...", version_clone.version);
            println!("-> URL: {url}");

            let cache_dir = dirs::home_dir()
                .ok_or_else(|| DownloadError::Io("Cannot get home directory".to_string()))?
                .join(".fnva")
                .join("cache")
                .join("downloads");

            tokio::fs::create_dir_all(&cache_dir)
                .await
                .map_err(|e| DownloadError::Io(format!("Failed to create cache directory: {e}")))?;

            let mirror_name = mirrors.first().map(|m| m.name.as_str()).unwrap_or("unknown");
            let extension = platform_clone.archive_ext();
            let file_name = format!(
                "OpenJDK-{}-{}.{}-{}.{}",
                version_clone.version, platform_clone.os, platform_clone.arch, mirror_name, extension
            );
            let file_path = cache_dir.join(&file_name);

            if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                if metadata.len() > 0 {
                    println!("-> Using cached file: {} MB", metadata.len() / (1024 * 1024));
                    let canonical = file_path.canonicalize()
                        .map_err(|e| DownloadError::Io(format!("Path canonicalization failed: {e}")))?;
                    return Ok(DownloadTarget::File(
                        canonical.to_str().ok_or_else(|| DownloadError::Io("Invalid path encoding".to_string()))?.to_string()
                    ));
                }
            }

            download_to_file(&self.client, &url, &file_path, |c, t| progress_callback(c, t))
                .await
                .map_err(|e| DownloadError::from(format!("Download failed: {e}")))?;

            let file_size = tokio::fs::metadata(&file_path)
                .await.map_err(|e| DownloadError::Io(format!("Failed to get file size: {e}")))?.len();
            println!("<- Download complete: {} MB", file_size / (1024 * 1024));

            let canonical = file_path.canonicalize()
                .map_err(|e| DownloadError::Io(format!("Path canonicalization failed: {e}")))?;
            Ok(DownloadTarget::File(
                canonical.to_str().ok_or_else(|| DownloadError::Io("Invalid path encoding".to_string()))?.to_string()
            ))
        })
    }
}
