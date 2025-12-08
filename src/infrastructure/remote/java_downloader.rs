use super::platform::Platform;
use super::UnifiedJavaVersion;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

pub enum DownloadTarget {
    Bytes(Vec<u8>),
    File(String),
}

#[derive(Debug)]
pub enum DownloadError {
    Network(String),
    NotFound,
    Invalid(String),
    Io(String),
    VersionParse,
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadError::Network(msg) => write!(f, "Network error: {msg}"),
            DownloadError::NotFound => write!(f, "Resource not found"),
            DownloadError::Invalid(msg) => write!(f, "Invalid data: {msg}"),
            DownloadError::Io(msg) => write!(f, "IO error: {msg}"),
            DownloadError::VersionParse => write!(f, "Version parse error"),
        }
    }
}

impl std::error::Error for DownloadError {}

impl From<String> for DownloadError {
    fn from(err: String) -> Self {
        DownloadError::Network(err)
    }
}

pub trait JavaDownloader: Send + Sync {
    fn list_available_versions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<UnifiedJavaVersion>, DownloadError>> + Send + '_>>;

    fn find_version_by_spec(
        &self,
        spec: &str,
    ) -> Pin<Box<dyn Future<Output = Result<UnifiedJavaVersion, DownloadError>> + Send + '_>>;

    fn get_download_url(
        &self,
        version: &UnifiedJavaVersion,
        platform: &Platform,
    ) -> Pin<Box<dyn Future<Output = Result<String, DownloadError>> + Send + '_>>;

    fn download_java(
        &self,
        version: &UnifiedJavaVersion,
        platform: &Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send + Sync>,
    ) -> Pin<Box<dyn Future<Output = Result<DownloadTarget, DownloadError>> + Send + '_>>;
}
