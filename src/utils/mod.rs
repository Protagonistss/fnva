pub mod validation;
pub mod filesystem;
pub mod env_vars;
pub mod paths;

pub use validation::*;
pub use filesystem::*;
pub use env_vars::*;
pub use paths::*;

// 重新导出 Java 验证函数以保持向后兼容
pub use validation::validate_java_home;