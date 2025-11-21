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

// 具体类型导出以保持API可用性
pub use remote_manager::{RemoteManager, JavaVersionInfo, MavenVersionInfo, MavenArtifactInfo};
pub use github_downloader::{GitHubJavaDownloader, GitHubJavaVersion, GitHubJavaRelease, GitHubAsset};
pub use aliyun_downloader::{AliyunJavaDownloader, AliyunJavaVersion};
pub use tsinghua_downloader::{TsinghuaJavaDownloader, TsinghuaJavaVersion};
pub use java_downloader::{JavaDownloader, DownloadTarget, DownloadError};
pub use version_registry::{VersionRegistry, RegistryEntry};
