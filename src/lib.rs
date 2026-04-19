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
pub use core::session::*;
pub use core::switcher::*;

// 向后兼容的重新导出
pub use infrastructure::config::{CcEnvironment, Config, JavaEnvironment, LlmEnvironment};
pub use infrastructure::shell::ShellType;
