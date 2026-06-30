//! 泛化的工具下载器 trait —— 取代 Java 专属的 `JavaDownloader`。
//!
//! 入参从 `UnifiedJavaVersion` 改为通用的 [`ResolvedVersion`],方法名
//! `download_java` → `download`。`DownloadTarget` / `DownloadError` 复用
//! `remote::java_downloader` 的现有定义(本属通用类型,只是历史地放在了
//! Java 文件里)。

use crate::infrastructure::remote::java_downloader::{DownloadError, DownloadTarget};
use crate::infrastructure::remote::platform::Platform;
use std::future::Future;
use std::pin::Pin;

use super::version_discovery::ResolvedVersion;

/// 工具无关的下载器接口。
pub trait ToolDownloader: Send + Sync {
    fn list_available_versions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ResolvedVersion>, DownloadError>> + Send + '_>>;

    fn find_version_by_spec(
        &self,
        spec: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion, DownloadError>> + Send + '_>>;

    fn get_download_url(
        &self,
        version: &ResolvedVersion,
        platform: &Platform,
    ) -> Pin<Box<dyn Future<Output = Result<String, DownloadError>> + Send + '_>>;

    fn download(
        &self,
        version: &ResolvedVersion,
        platform: &Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Result<DownloadTarget, DownloadError>> + Send + '_>>;
}
