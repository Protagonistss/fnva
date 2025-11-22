// 核心模块
pub mod cli;
pub mod core;
pub mod environments;
pub mod error;
pub mod infrastructure;
pub mod utils;

// 重新导出常用类型（向后兼容）
pub use cli::*;
pub use environments::*;
pub use error::*;
pub use infrastructure::*;
pub use utils::*;
// 明确导出 core 模块，先导出 infrastructure 避免常量模块名冲突
pub use core::environment_manager::*;
pub use core::error_messages::*;
pub use core::session::*;
pub use core::switcher::*;
// 使用命名空间导入常量，避免冲突
pub use core::constants as app_constants;

// 向后兼容的重新导出
pub use infrastructure::config::{Config, JavaEnvironment, LlmEnvironment};
pub use infrastructure::network::NetworkTester;
pub use infrastructure::remote::{
    JavaVersionInfo, MavenArtifactInfo, MavenVersionInfo, RemoteManager,
};
pub use infrastructure::shell::ShellType;
