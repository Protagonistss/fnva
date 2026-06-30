//! 工具描述符 —— 一种工具的元信息(安装子目录、资产模型、home 定位/校验)。
//!
//! 每个工具一份 `const` 实例(如 Java 侧的 `JAVA_DESCRIPTOR`),供通用
//! installer 骨架参数化工具差异。

use std::path::Path;

/// 工具标识(编译期已知的工具 id,如 `"java"` / `"maven"`)
pub type ToolId = &'static str;

/// 资产模型:决定「按平台分文件」还是「单包跨平台」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetModel {
    /// 每个平台一个不同文件名(Java / Node),实际文件名从版本注册表取。
    PerPlatform,
    /// 跨平台单包,文件名仅由 version 决定(Maven / Gradle)。
    SingleArchive,
}

/// 工具元信息
pub struct ToolDescriptor {
    pub id: ToolId,
    pub display_name: &'static str,
    pub asset_model: AssetModel,
    /// 安装子目录名(如 `"packages/java"` / `"packages/maven"`)
    pub install_subdir: &'static str,
    /// 校验 home 目录是否合法(如存在 `bin/java` 或 `bin/mvn`)
    pub home_validator: fn(&str) -> bool,
    /// 给定解压根目录,返回实际 home:
    /// Java 需查找 macOS 的 `Contents/Home` 或子目录;Maven 直接是解压根。
    pub locate_home: fn(&Path) -> Result<String, String>,
}
