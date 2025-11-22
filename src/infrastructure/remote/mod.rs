pub mod repositories;
pub mod remote_manager;
pub mod github_downloader;
pub mod aliyun_downloader;
pub mod platform;
pub mod download;
pub mod tsinghua_downloader;
pub mod cache;
pub mod java_downloader;
pub mod version_registry;

pub use repositories::*;
pub use remote_manager::*;
pub use platform::Platform;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// 具体类型导出以保持API可用性
pub use remote_manager::{RemoteManager, JavaVersionInfo, MavenVersionInfo, MavenArtifactInfo};
pub use github_downloader::{GitHubJavaDownloader, GitHubJavaRelease, GitHubAsset};
pub use aliyun_downloader::AliyunJavaDownloader;
pub use tsinghua_downloader::TsinghuaJavaDownloader;
pub use java_downloader::{JavaDownloader, DownloadTarget, DownloadError};
pub use version_registry::{VersionRegistry, RegistryEntry};

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
    pub download_urls: HashMap<String, DownloadSource>, // os-arch -> source
    pub is_lts: bool,
    pub published_at: String,
    pub checksums: Option<HashMap<String, String>>, // os-arch -> checksum
}
