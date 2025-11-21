use super::platform::Platform;
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
}

impl From<String> for DownloadError {
    fn from(err: String) -> Self {
        DownloadError::Network(err)
    }
}

pub trait JavaDownloader {
    type Version;
    fn version_string(&self, version: &Self::Version) -> String;
    fn list_available_versions(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Self::Version>, DownloadError>> + Send + '_>>;
    fn find_version_by_spec(
        &self,
        spec: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Version, DownloadError>> + Send + '_>>;
    fn get_download_url(
        &self,
        version: &Self::Version,
        platform: &Platform,
    ) -> Pin<Box<dyn Future<Output = Result<String, DownloadError>> + Send + '_>>;
    fn download_java(
        &self,
        version: &Self::Version,
        platform: &Platform,
        progress_callback: Box<dyn Fn(u64, u64) + Send>,
    ) -> Pin<Box<dyn Future<Output = Result<DownloadTarget, DownloadError>> + Send + '_>>;
}
