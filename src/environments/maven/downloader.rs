//! Maven 下载器:`MirrorDirectoryDiscovery` + `MirrorResolver`,
//! 复用通用的 [`GenericDownloader`](crate::infrastructure::tool_protocol::GenericDownloader)。
//! Maven 是跨平台单包,缓存文件名固定 `apache-maven-<version>-<mirror>.tar.gz`。

use crate::environments::maven::version_discovery::MirrorDirectoryDiscovery;
use crate::infrastructure::config::MirrorConfig;
use crate::infrastructure::tool_protocol::generic_downloader::GenericDownloader;

/// Maven 下载器(通用下载器的 Maven 实例化)。
pub type MavenDownloader = GenericDownloader<MirrorDirectoryDiscovery>;

impl MavenDownloader {
    pub fn new(mirrors: Vec<MirrorConfig>) -> Self {
        Self::with_file_name(
            MirrorDirectoryDiscovery::new(),
            mirrors,
            |version, mirror| format!("apache-maven-{}-{}.tar.gz", version.version, mirror),
        )
    }
}
