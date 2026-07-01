//! Tool Source Protocol —— 工具无关的「镜像源 + 模板 + 版本发现 + 下载」协议。
//!
//! 把原本 Java 专属的镜像解析逻辑泛化,使 Java / Maven(及未来 Gradle/Node)
//! 共用同一套协议。
//!
//! - [`TemplateVars`] / [`MirrorResolver`]:模板渲染 + 镜像回退(阶段 0)
//! - [`ResolvedVersion`] / [`VersionDiscovery`] / [`EmbeddedRegistryDiscovery`]:版本发现(阶段 1)
//! - [`ToolDownloader`]:泛化下载器 trait(阶段 1)
//! - `ToolDescriptor`:工具元信息(阶段 2 接入 installer 时引入)

pub mod descriptor;
pub mod downloader;
pub mod mirror_resolver;
pub mod template_vars;
pub mod version_discovery;

pub use descriptor::{AssetModel, ToolDescriptor, ToolId};
pub use downloader::ToolDownloader;
pub use mirror_resolver::{MirrorResolver, ResolveError};
pub use template_vars::TemplateVars;
pub use version_discovery::{DiscoveryError, ResolvedVersion, VersionDiscovery};
