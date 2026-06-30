//! Java 下载器:`AdoptiumDiscovery` + `MirrorResolver` 组合。
//! 复用 Maven [`MavenDownloader`](crate::environments::maven::downloader) 的模式。

use crate::infrastructure::config::MirrorConfig;
use crate::infrastructure::remote::download::download_to_file;
use crate::infrastructure::remote::java_downloader::{DownloadError, DownloadTarget};
use crate::infrastructure::remote::platform::Platform;
use crate::infrastructure::tool_protocol::{
    MirrorResolver, ResolvedVersion, ToolDownloader, VersionDiscovery,
};
use std::future::Future;
use std::pin::Pin;

use super::version_discovery::AdoptiumDiscovery;

/// Java 下载器
pub struct JavaDownloader {
    discovery: AdoptiumDiscovery,
    resolver: MirrorResolver,
}

impl JavaDownloader {
    pub fn new(mirrors: Vec<MirrorConfig>) -> Self {
        Self {
            discovery: AdoptiumDiscovery::new(),
            resolver: MirrorResolver::new(mirrors),
        }
    }

    /// 从 filename 推断归档扩展名(.tar.gz / .zip)。
    fn ext_of(filename: &str) -> &'static str {
        if filename.ends_with(".zip") {
            "zip"
        } else {
            "tar.gz"
        }
    }

    /// Force re-fetch the remote version cache.
    pub async fn refresh(&self) -> Result<(), DownloadError> {
        self.discovery
            .refresh()
            .await
            .map_err(|e| DownloadError::from(format!("{e}")))
    }
}

impl ToolDownloader for JavaDownloader {
    fn list_available_versions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ResolvedVersion>, DownloadError>> + Send + '_>> {
        Box::pin(async {
            self.discovery
                .list()
                .await
                .map_err(|e| DownloadError::from(format!("{e}")))
        })
    }

    fn find_version_by_spec(
        &self,
        spec: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion, DownloadError>> + Send + '_>> {
        let s = spec.to_string();
        Box::pin(async move {
            self.discovery
                .find(&s)
                .await
                .map_err(|e| DownloadError::from(format!("{e}")))
        })
    }

    fn get_download_url(
        &self,
        version: &ResolvedVersion,
        _platform: &Platform,
    ) -> Pin<Box<dyn Future<Output = Result<String, DownloadError>> + Send + '_>> {
        let vars = version.template_vars.clone();
        Box::pin(async move {
            self.resolver
                .resolve(&vars)
                .await
                .map_err(|e| DownloadError::from(e.to_string()))
        })
    }

    fn download(
        &self,
        version: &ResolvedVersion,
        _platform: &Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Result<DownloadTarget, DownloadError>> + Send + '_>> {
        let vars = version.template_vars.clone();
        let version_str = version.version.clone();
        let os = vars.os.clone();
        let arch = vars.arch.clone();
        let ext = Self::ext_of(&vars.filename).to_string();
        Box::pin(async move {
            let url = self
                .resolver
                .resolve(&vars)
                .await
                .map_err(|e| DownloadError::from(e.to_string()))?;
            println!("Downloading Java {version_str}...");
            println!("URL: {url}");

            let cache_dir = crate::infrastructure::paths::downloads_dir()
                .map_err(|e| DownloadError::Io(format!("Cannot get downloads dir: {e}")))?;
            tokio::fs::create_dir_all(&cache_dir)
                .await
                .map_err(|e| DownloadError::Io(format!("Failed to create cache directory: {e}")))?;

            let mirror_name = self.resolver.first_mirror_name().to_string();
            let file_name = format!("OpenJDK-{version_str}-{os}.{arch}-{mirror_name}.{ext}");
            let file_path = cache_dir.join(&file_name);

            if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                if metadata.len() > 0 {
                    println!("Using cached file ({} MB)", metadata.len() / (1024 * 1024));
                    let canonical = file_path
                        .canonicalize()
                        .map_err(|e| DownloadError::Io(format!("Path canonicalization failed: {e}")))?;
                    return Ok(DownloadTarget::File(
                        canonical
                            .to_str()
                            .ok_or_else(|| DownloadError::Io("Invalid path encoding".to_string()))?
                            .to_string(),
                    ));
                }
            }

            download_to_file(self.resolver.client(), &url, &file_path, |c, t| {
                progress_callback(c, t)
            })
            .await
            .map_err(|e| DownloadError::from(format!("Download failed: {e}")))?;

            let file_size = tokio::fs::metadata(&file_path)
                .await
                .map_err(|e| DownloadError::Io(format!("Failed to get file size: {e}")))?
                .len();
            println!("Download complete ({} MB)", file_size / (1024 * 1024));

            let canonical = file_path
                .canonicalize()
                .map_err(|e| DownloadError::Io(format!("Path canonicalization failed: {e}")))?;
            Ok(DownloadTarget::File(
                canonical
                    .to_str()
                    .ok_or_else(|| DownloadError::Io("Invalid path encoding".to_string()))?
                    .to_string(),
            ))
        })
    }
}
