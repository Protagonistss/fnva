//! 工具无关的通用下载器:组合 [`VersionDiscovery`] + [`MirrorResolver`] + 可定制的文件名策略。
//!
//! 取代 Java / Maven 各自重复的 `list_available_versions` / `find_version_by_spec` /
//! `get_download_url` / `download` 实现 —— 这四个方法对两类工具完全一致,唯一差异是
//! `download` 时缓存文件名的拼法,通过构造时传入的闭包注入。

use crate::infrastructure::config::MirrorConfig;
use crate::infrastructure::remote::download::download_with_cache;
use crate::infrastructure::remote::java_downloader::{DownloadError, DownloadTarget};
use crate::infrastructure::remote::platform::Platform;
use std::future::Future;
use std::pin::Pin;

use super::{MirrorResolver, ResolvedVersion, ToolDownloader, VersionDiscovery};

type FileNameFn = Box<dyn Fn(&ResolvedVersion, &str) -> String + Send + Sync>;

/// 通用下载器:版本发现策略 + 镜像解析器 + 文件名生成函数。
pub struct GenericDownloader<D: VersionDiscovery> {
    discovery: D,
    resolver: MirrorResolver,
    file_name: FileNameFn,
}

impl<D: VersionDiscovery> GenericDownloader<D> {
    /// 用指定的版本发现策略、镜像列表与文件名生成函数构造下载器。
    /// `file_name` 接收已解析版本与首个可用镜像名,返回缓存文件名。
    pub fn with_file_name<F>(discovery: D, mirrors: Vec<MirrorConfig>, file_name: F) -> Self
    where
        F: Fn(&ResolvedVersion, &str) -> String + Send + Sync + 'static,
    {
        Self {
            discovery,
            resolver: MirrorResolver::new(mirrors),
            file_name: Box::new(file_name),
        }
    }

    /// 强制重新拉取远端版本缓存(若发现策略支持)。
    pub async fn refresh(&self) -> Result<(), DownloadError> {
        self.discovery
            .refresh()
            .await
            .map_err(|e| DownloadError::from(format!("{e}")))
    }
}

impl<D: VersionDiscovery> ToolDownloader for GenericDownloader<D> {
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
        let version_clone = version.clone();
        Box::pin(async move {
            let url = self
                .resolver
                .resolve(&vars)
                .await
                .map_err(|e| DownloadError::from(e.to_string()))?;
            let mirror_name = self.resolver.first_mirror_name().to_string();
            let file_name = (self.file_name)(&version_clone, &mirror_name);
            download_with_cache(self.resolver.client(), &url, &file_name, progress_callback)
                .await
                .map_err(|e| DownloadError::from(e.to_string()))
        })
    }
}
