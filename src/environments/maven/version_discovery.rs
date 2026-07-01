//! Maven 版本发现:抓取 Apache archive 的 `maven-3/` 目录列表,解析出版本号。
//!
//! 清华镜像的 `maven-3/` 目录只保留最新版,不能用作 list 源;因此
//! `discovery_url` 固定指向 apache archive(完整历史)。清华源只作下载加速镜像
//! (见 `MavenInstaller` 的镜像配置)。
//!
//! 抓取结果缓存到 `~/.fnva/maven_versions.json`,带 24h TTL,支持
//! `fnva maven refresh` 强制刷新。抓取失败时回退编译期嵌入的兜底列表。

use crate::infrastructure::tool_protocol::template_vars::TemplateVars;
use crate::infrastructure::tool_protocol::version_discovery::{
    DiscoveryError, ResolvedVersion, VersionDiscovery,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

const ARCHIVE_URL: &str = "https://archive.apache.org/dist/maven/maven-3/";
const CACHE_TTL_SECS: i64 = 86_400; // 24h

/// 缓存文件结构(`~/.fnva/maven_versions.json`)
#[derive(Serialize, Deserialize)]
struct VersionCache {
    /// 抓取时间(unix 秒)
    fetched_at: i64,
    versions: Vec<String>,
}

/// 镜像目录动态发现:Maven 版本来源。
pub struct MirrorDirectoryDiscovery {
    discovery_url: &'static str,
    client: Client,
}

impl MirrorDirectoryDiscovery {
    pub fn new() -> Self {
        Self {
            discovery_url: ARCHIVE_URL,
            client: Client::new(),
        }
    }

    fn cache_path() -> Result<std::path::PathBuf, DiscoveryError> {
        crate::infrastructure::paths::maven_versions_path().map_err(DiscoveryError::Io)
    }

    /// 手写扫描目录 HTML,提取 `href="X.Y.Z/"` 形态的纯数字版本号。
    ///
    /// 不依赖 class / 标签结构,只认 `href="<三段纯数字>/"`,宽松且抗变动。
    fn parse_directory_html(html: &str) -> Vec<String> {
        let mut versions = Vec::new();
        let mut rest = html;
        while let Some(pos) = rest.find("href=\"") {
            rest = &rest[pos + 6..]; // skip `href="`
                                     // 候选版本号:读到 `"` 或 `/`
            let end = rest.find(['"', '/']).unwrap_or(rest.len());
            let candidate = &rest[..end];
            if Self::is_numeric_version(candidate) {
                let c = candidate.to_string();
                if !versions.contains(&c) {
                    versions.push(c);
                }
            }
            // 推进到本 href 值的结束引号之后
            match rest.find('"') {
                Some(np) => rest = &rest[np + 1..],
                None => break,
            }
        }
        versions
    }

    /// 校验 `X.Y.Z` 三段纯数字
    fn is_numeric_version(s: &str) -> bool {
        let parts: Vec<&str> = s.split('.').collect();
        parts.len() == 3
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
    }

    /// 抓取目录并写缓存。网络错误向上传播(由 `load_versions` / `refresh`
    /// 决定是否回退);抓到内容但解析为空则回退嵌入式列表。
    async fn fetch_and_cache(&self) -> Result<Vec<String>, DiscoveryError> {
        let html = self
            .client
            .get(self.discovery_url)
            .send()
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?
            .text()
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;
        let mut versions = Self::parse_directory_html(&html);
        if versions.is_empty() {
            return Self::embedded_versions();
        }
        versions.sort_by_key(|v| std::cmp::Reverse(version_sort_key(v)));

        if let Ok(path) = Self::cache_path() {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let cache = VersionCache {
                fetched_at: chrono::Utc::now().timestamp(),
                versions: versions.clone(),
            };
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = std::fs::write(&path, json);
            }
        }
        Ok(versions)
    }

    /// TTL 内用本地缓存,否则抓取;抓取失败(离线)回退嵌入式列表。
    async fn load_versions(&self) -> Result<Vec<String>, DiscoveryError> {
        if let Ok(path) = Self::cache_path() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cache) = serde_json::from_str::<VersionCache>(&content) {
                    if chrono::Utc::now().timestamp() - cache.fetched_at < CACHE_TTL_SECS {
                        return Ok(cache.versions);
                    }
                }
            }
        }
        match self.fetch_and_cache().await {
            Ok(v) => Ok(v),
            Err(_) => Self::embedded_versions(),
        }
    }

    /// 编译期嵌入的兜底版本列表(`config/maven_versions.toml`)。
    fn embedded_versions() -> Result<Vec<String>, DiscoveryError> {
        const EMBEDDED: &str = include_str!("../../../config/maven_versions.toml");
        let parsed: toml::Value =
            toml::from_str(EMBEDDED).map_err(|e| DiscoveryError::Parse(e.to_string()))?;
        let arr = parsed
            .get("versions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                DiscoveryError::Parse("no versions array in maven_versions.toml".into())
            })?;
        let mut vs: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        vs.sort_by_key(|v| std::cmp::Reverse(version_sort_key(v)));
        Ok(vs)
    }
}

impl Default for MirrorDirectoryDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// 语义版本排序键:`X.Y.Z` → `[X, Y, Z]`(数值比较)。
/// 避免字符串序下 `"3.9.9" > "3.9.16"` 的错误。
fn version_sort_key(v: &str) -> Vec<u64> {
    v.split('.').filter_map(|p| p.parse::<u64>().ok()).collect()
}

fn make_resolved(version: &str) -> ResolvedVersion {
    ResolvedVersion {
        version: version.to_string(),
        major: None,
        is_lts: false,
        display: format!("Apache Maven {version}"),
        template_vars: TemplateVars {
            version: version.to_string(),
            filename: format!("apache-maven-{version}-bin.tar.gz"),
            ..Default::default()
        },
    }
}

impl VersionDiscovery for MirrorDirectoryDiscovery {
    fn list(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ResolvedVersion>, DiscoveryError>> + Send + '_>>
    {
        Box::pin(async {
            let versions = self.load_versions().await?;
            Ok(versions.iter().map(|v| make_resolved(v)).collect())
        })
    }

    fn find(
        &self,
        spec: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion, DiscoveryError>> + Send + '_>> {
        let s = spec.to_string();
        Box::pin(async move {
            let versions = self.load_versions().await?;
            let cleaned = s.trim().to_lowercase();
            if cleaned == "latest" || cleaned == "newest" {
                return versions
                    .first()
                    .map(|v| make_resolved(v))
                    .ok_or_else(|| DiscoveryError::NotFound(s.clone()));
            }
            // 精确匹配
            if let Some(v) = versions.iter().find(|v| v.as_str() == s.as_str()) {
                return Ok(make_resolved(v));
            }
            // 前缀匹配(如 "3.9")
            if let Some(v) = versions.iter().find(|v| v.starts_with(s.as_str())) {
                return Ok(make_resolved(v));
            }
            Err(DiscoveryError::NotFound(s))
        })
    }

    fn supports_refresh(&self) -> bool {
        true
    }

    fn refresh(&self) -> Pin<Box<dyn Future<Output = Result<(), DiscoveryError>> + Send + '_>> {
        Box::pin(async { self.fetch_and_cache().await.map(|_| ()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_directory_extracts_numeric_versions() {
        let html = r#"
        <html><body>
        <a href="3.9.16/">3.9.16/</a>
        <a href="3.9.9/">3.9.9/</a>
        <a href="3.8.8/">3.8.8/</a>
        <a href="../">Parent Directory</a>
        <a href="?C=N;O=D">Name</a>
        <a href="?C=M;O=A">Last modified</a>
        </body></html>
        "#;
        let v = MirrorDirectoryDiscovery::parse_directory_html(html);
        assert!(v.contains(&"3.9.16".to_string()));
        assert!(v.contains(&"3.9.9".to_string()));
        assert!(v.contains(&"3.8.8".to_string()));
        // 不应混入非版本条目
        assert!(!v.iter().any(|x| x == ".."));
        assert!(v.len() == 3);
    }

    #[test]
    fn is_numeric_version_validates() {
        assert!(MirrorDirectoryDiscovery::is_numeric_version("3.9.16"));
        assert!(!MirrorDirectoryDiscovery::is_numeric_version("3.9"));
        assert!(!MirrorDirectoryDiscovery::is_numeric_version(".."));
        assert!(!MirrorDirectoryDiscovery::is_numeric_version("3.9.16-beta"));
        assert!(!MirrorDirectoryDiscovery::is_numeric_version(""));
    }

    #[test]
    fn embedded_versions_loads_and_contains_known() {
        let v = MirrorDirectoryDiscovery::embedded_versions().expect("embedded load");
        assert!(v.len() >= 3);
        assert!(v.iter().any(|x| x == "3.9.16"));
        // 倒序(最新优先)
        assert_eq!(v.first().map(String::as_str), Some("3.9.16"));
    }

    #[test]
    fn supports_refresh_is_true() {
        assert!(MirrorDirectoryDiscovery::new().supports_refresh());
    }
}
