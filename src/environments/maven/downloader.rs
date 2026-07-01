//! Maven 下载器:版本来源用 [`MirrorDirectoryDiscovery`],下载用 [`MirrorResolver`]
//! 解析镜像 URL。Maven 是跨平台单包,`download`/`get_download_url` 忽略平台。

use crate::infrastructure::config::MirrorConfig;
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
            let mirror_name = self.resolver.first_mirror_name().to_string();
            let file_name = format!("apache-maven-{version_str}-{mirror_name}.tar.gz");

            crate::infrastructure::remote::download::download_with_cache(
                self.resolver.client(),
                &url,
                &file_name,
                progress_callback,
            )
            .await
            .map_err(|e| DownloadError::from(e.to_string()))
        })
    }
}
