//! 版本发现策略(可插拔)。
//!
//! 不同工具的「版本列表从哪来」差异很大:Java 抓清华 Adoptium 镜像目录,
//! Maven 抓 apache archive 目录。本 trait 把这个差异抽象成策略,下载器与
//! 安装器只依赖 [`VersionDiscovery`],不关心具体来源。

use crate::infrastructure::tool_protocol::template_vars::TemplateVars;
use std::future::Future;
use std::pin::Pin;

/// 版本发现错误
#[derive(Debug)]
pub enum DiscoveryError {
    Network(String),
    NotFound(String),
    Parse(String),
    Io(String),
    /// 该发现策略不支持 refresh
    RefreshUnsupported,
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::Network(msg) => write!(f, "Network error: {msg}"),
            DiscoveryError::NotFound(spec) => write!(f, "Version '{spec}' not found"),
            DiscoveryError::Parse(msg) => write!(f, "Parse error: {msg}"),
            DiscoveryError::Io(msg) => write!(f, "IO error: {msg}"),
            DiscoveryError::RefreshUnsupported => write!(f, "Refresh not supported by this source"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// 一个已解析的、可直接用于下载/渲染的通用版本。
#[derive(Debug, Clone)]
pub struct ResolvedVersion {
    pub version: String,
    pub major: Option<u32>,
    pub is_lts: bool,
    pub display: String,
    pub template_vars: TemplateVars,
}

/// 版本发现策略
pub trait VersionDiscovery: Send + Sync {
    fn list(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ResolvedVersion>, DiscoveryError>> + Send + '_>>;

    fn find(
        &self,
        spec: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion, DiscoveryError>> + Send + '_>>;

    fn supports_refresh(&self) -> bool {
        false
    }

    fn refresh(&self) -> Pin<Box<dyn Future<Output = Result<(), DiscoveryError>> + Send + '_>> {
        Box::pin(async { Err(DiscoveryError::RefreshUnsupported) })
    }
}
