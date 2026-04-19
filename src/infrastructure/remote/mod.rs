pub mod cache;
pub mod download;
pub mod java_downloader;
pub mod mirror_utils;
pub mod platform;
pub mod template_downloader;
pub mod version_registry;

pub use platform::Platform;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use java_downloader::{DownloadError, DownloadTarget, JavaDownloader};
pub use template_downloader::TemplateDownloader;
pub use version_registry::{RegistryEntry, VersionRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadSource {
    pub primary: String,
    pub fallback: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedJavaVersion {
    pub version: String,
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub release_name: String,
    pub tag_name: String,
    pub download_urls: HashMap<String, DownloadSource>,
    pub is_lts: bool,
    pub published_at: String,
    pub checksums: Option<HashMap<String, String>>,
}
