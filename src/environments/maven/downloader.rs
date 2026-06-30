//! Maven 下载器:版本来源用 [`MirrorDirectoryDiscovery`],下载用 [`MirrorResolver`]
//! 解析镜像 URL。Maven 是跨平台单包,`download`/`get_download_url` 忽略平台。

use crate::infrastructure::config::MirrorConfig;
use crate::infrastructure::remote::download::download_to_file;
use crate::infrastructure::remote::java_downloader::{DownloadError, DownloadTarget};
use crate::infrastructure::remote::platform::Platform;
use crate::infrastructure::tool_protocol::{
    MirrorResolver, ResolvedVersion, ToolDownloader, VersionDiscovery,
};
use std::future::Future;
use std::pin::Pin;

use super::version_discovery::MirrorDirectoryDiscovery;

/// Maven 下载器
pub struct MavenDownloader {
    discovery: MirrorDirectoryDiscovery,
    resolver: MirrorResolver,
}

impl MavenDownloader {
    pub fn new(mirrors: Vec<MirrorConfig>) -> Self {
        Self {
            discovery: MirrorDirectoryDiscovery::new(),
            resolver: MirrorResolver::new(mirrors),
        }
    }
}

impl ToolDownloader for MavenDownloader {
    fn list_available_versions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ResolvedVersion>, DownloadError>> + Send + '_>>
    {
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
        Box::pin(async move {
            let url = self
                .resolver
                .resolve(&vars)
                .await
                .map_err(|e| DownloadError::from(e.to_string()))?;
            crate::cli::print::step("Source", &url);

            let cache_dir =
                crate::infrastructure::paths::downloads_dir().map_err(DownloadError::Io)?;
            tokio::fs::create_dir_all(&cache_dir)
                .await
                .map_err(|e| DownloadError::Io(format!("Failed to create cache directory: {e}")))?;

            let mirror_name = self.resolver.first_mirror_name().to_string();
            let file_name = format!("apache-maven-{version_str}-{mirror_name}.tar.gz");
            let file_path = cache_dir.join(&file_name);

            if let Ok(metadata) = tokio::fs::metadata(&file_path).await {
                if metadata.len() > 0 {
                    crate::cli::print::step("Status", &format!("Using cached file ({} MB)", metadata.len() / (1024 * 1024)));
                    let canonical = file_path.canonicalize().map_err(|e| {
                        DownloadError::Io(format!("Path canonicalization failed: {e}"))
                    })?;
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
            crate::cli::print::step("Status", &format!("Download complete ({} MB)", file_size / (1024 * 1024)));

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
