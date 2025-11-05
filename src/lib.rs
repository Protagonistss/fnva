pub mod config;
pub mod installer;
pub mod java;
pub mod llm;
pub mod network_test;
pub mod package_manager;
pub mod platform;
pub mod remote;
pub mod shell_hook;
pub mod shell_integration;
pub mod utils;

// 重新导出常用类型
pub use config::{Config, JavaEnvironment, LlmEnvironment};
pub use installer::JavaInstaller;
pub use java::{JavaManager, JavaInstallation};
pub use llm::LlmManager;
pub use network_test::NetworkTester;
pub use package_manager::JavaPackageManager;
pub use remote::{RemoteManager, JavaVersionInfo, MavenVersionInfo, MavenArtifactInfo};
pub use shell_hook::ShellHook;
pub use shell_integration::ShellIntegration;

