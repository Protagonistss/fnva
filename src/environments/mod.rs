pub mod java;
pub mod llm;
pub mod cc;

// 具体类型导入以避免ambiguous glob re-exports
pub use java::{manager::JavaManager, environment_manager::JavaEnvironmentManager, version_manager::{VersionManager, VersionSpec, JavaVersion}};
pub use llm::{manager::LlmManager, environment_manager::LlmEnvironmentManager, providers::LlmProviderAsync};
pub use cc::environment_manager::*;