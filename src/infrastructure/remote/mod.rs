pub mod http_client;
pub mod repositories;
pub mod remote_manager;
pub mod github_downloader;
pub mod aliyun_downloader;

pub use http_client::*;
pub use repositories::*;
pub use remote_manager::*;
pub use github_downloader::*;
pub use aliyun_downloader::*;

// 类型别名以保持向后兼容
pub use remote_manager::RemoteManager;
pub use remote_manager::JavaVersionInfo;
pub use remote_manager::MavenVersionInfo;
pub use remote_manager::MavenArtifactInfo;
pub use github_downloader::{GitHubJavaDownloader, GitHubJavaVersion, GitHubJavaRelease, GitHubAsset};
pub use aliyun_downloader::{AliyunJavaDownloader, AliyunJavaVersion};