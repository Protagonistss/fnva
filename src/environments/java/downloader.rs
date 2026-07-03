//! Java 下载器:`AdoptiumDiscovery` + `MirrorResolver` 组合,
//! 复用通用的 [`GenericDownloader`](crate::infrastructure::tool_protocol::GenericDownloader)。

use crate::environments::java::version_discovery::AdoptiumDiscovery;
use crate::infrastructure::config::MirrorConfig;
use crate::infrastructure::tool_protocol::generic_downloader::GenericDownloader;

/// Java 下载器(通用下载器的 Java 实例化)。
pub type JavaDownloader = GenericDownloader<AdoptiumDiscovery>;

impl JavaDownloader {
    pub fn new(mirrors: Vec<MirrorConfig>) -> Self {
        Self::with_file_name(AdoptiumDiscovery::new(), mirrors, |version, mirror| {
            let vars = &version.template_vars;
            let ext = if vars.filename.ends_with(".zip") {
                "zip"
            } else {
                "tar.gz"
            };
            format!(
                "OpenJDK-{}-{}.{}-{}.{}",
                version.version, vars.os, vars.arch, mirror, ext
            )
        })
    }
}
