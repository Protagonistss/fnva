// 核心模块
pub mod core;
pub mod cli;
pub mod environments;
pub mod infrastructure;
pub mod utils;
pub mod error;

// 重新导出常用类型（向后兼容）
pub use core::*;
pub use cli::*;
pub use environments::*;
pub use infrastructure::*;
pub use utils::*;
pub use error::*;

// 向后兼容的重新导出
pub use infrastructure::config::{Config, JavaEnvironment, LlmEnvironment};
pub use infrastructure::network::NetworkTester;
pub use infrastructure::remote::{RemoteManager, JavaVersionInfo, MavenVersionInfo, MavenArtifactInfo};
pub use infrastructure::shell::ShellType;

