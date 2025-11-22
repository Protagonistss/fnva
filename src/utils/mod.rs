pub mod env_vars;
pub mod filesystem;
pub mod paths;
pub mod validation;

pub use env_vars::*;
pub use filesystem::*;
pub use paths::*;
pub use validation::*;

// 重新导出 Java 验证函数以保持向后兼容
pub use validation::validate_java_home;
