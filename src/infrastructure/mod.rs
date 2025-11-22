pub mod config;
pub mod config_repository;
pub mod installer;
pub mod network;
pub mod remote;
pub mod shell;

pub use config::*;
pub use config_repository::*;
pub use installer::*;
pub use network::*;
// Platform 从 shell 模块导出（操作系统平台）
pub use shell::platform::*;
// Shell 模块其他导出
pub use shell::hook::*;
pub use shell::integration::*;
pub use shell::script_builder::*;
pub use shell::script_factory::*;
pub use shell::script_strategy::*;
// 明确导出 remote 平台
pub use remote::Platform as RemotePlatform;
pub use remote::*;
