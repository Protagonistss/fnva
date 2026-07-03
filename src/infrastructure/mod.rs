pub mod config;
pub mod installer;
pub mod paths;
pub mod remote;
pub mod scanner;
pub mod shell;
pub mod tool_protocol;

pub use config::*;
pub use installer::*;
// Platform 从 shell 模块导出（操作系统平台）
pub use shell::platform::*;
// Shell 模块其他导出
pub use shell::script_factory::*;
pub use shell::script_strategy::*;
// 明确导出 remote 平台
pub use remote::Platform as RemotePlatform;
pub use remote::*;
