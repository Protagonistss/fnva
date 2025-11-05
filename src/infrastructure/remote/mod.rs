pub mod http_client;
pub mod repositories;
pub mod remote_manager;

pub use http_client::*;
pub use repositories::*;
pub use remote_manager::*;

// 类型别名以保持向后兼容
pub use remote_manager::RemoteManager;
pub use remote_manager::JavaVersionInfo;
pub use remote_manager::MavenVersionInfo;
pub use remote_manager::MavenArtifactInfo;