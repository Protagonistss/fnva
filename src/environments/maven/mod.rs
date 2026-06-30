//! Maven 工具支持(版本发现 / 安装 / 环境切换)。
//!
//! 当前(阶段 3)仅含版本发现 [`MirrorDirectoryDiscovery`];安装器、
//! 环境管理器在后续阶段接入。

pub mod downloader;
pub mod environment_manager;
pub mod installer;
pub mod validator;
pub mod version_discovery;

pub use environment_manager::MavenEnvironmentManager;
pub use installer::MavenInstaller;
pub use version_discovery::MirrorDirectoryDiscovery;
