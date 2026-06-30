//! Java 版本发现:抓取清华 Adoptium 镜像目录,解析 filenames → 版本。
//!
//! 对齐 Maven 的 [`MirrorDirectoryDiscovery`](crate::environments::maven::version_discovery)
//! ——动态发现 + 本地缓存 + 嵌入兜底。清华 Adoptium 目录结构:
//! `Adoptium/{major}/jdk/{arch}/{os}/{filename}`。
//!
//! filename 格式:`OpenJDK{major}U-jdk_{arch}_{os}_hotspot_{ver}.tar.gz`
//! - major ≥ 9:`{ver}` = `{M}.{m}.{p}_{b}` → version `{M}.{m}.{p}+{b}`,tag `jdk-{version}`
//! - major 8:`{ver}` = `8u{u}b{b}` → version `8u{u}b{b}`,tag `jdk8u{u}-b{b}`

use crate::infrastructure::remote::platform::Platform;
use crate::infrastructure::tool_protocol::template_vars::TemplateVars;
use crate::infrastructure::tool_protocol::version_discovery::{
    DiscoveryError, ResolvedVersion, VersionDiscovery,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

const MIRROR_BASE: &str = "https://mirrors.tuna.tsinghua.edu.cn/Adoptium";
const CACHE_TTL_SECS: i64 = 86_400; // 24h
/// 支持发现的主版本(倒序,最新优先)
const SUPPORTED_MAJORS: &[u32] = &[25, 21, 17, 11, 8];
const LTS_MAJORS: &[u32] = &[25, 21, 17, 11, 8];

/// 版本规格解析结果(从 version_manager.rs 迁移)。
#[derive(Debug, Clone, PartialEq)]
pub enum VersionSpec {
    Major(u32),
    Exact(String),
    LatestLts,
    Latest,
    Range(u32, u32),
}

/// 解析版本规格字符串(从 version_manager.rs 迁移)。
pub fn parse_version_spec(spec: &str) -> Result<VersionSpec, String> {
    let spec_cleaned = spec.trim().to_lowercase();
    if spec_cleaned == "lts" || spec_cleaned == "latest-lts" {
        return Ok(VersionSpec::LatestLts);
    }
    if spec_cleaned == "latest" || spec_cleaned == "newest" {
        return Ok(VersionSpec::Latest);
    }
    let cleaned = spec_cleaned
        .replace('v', "")
        .replace("java", "")
        .replace("jdk", "")
        .replace("openjdk", "");
    if cleaned.contains('-') {
        let parts: Vec<&str> = cleaned.split('-').collect();
        if parts.len() == 2 {
            let start = parts[0]
                .parse::<u32>()
                .map_err(|_| format!("Invalid start version: {}", parts[0]))?;
            let end = parts[1]
                .parse::<u32>()
                .map_err(|_| format!("Invalid end version: {}", parts[1]))?;
            return Ok(VersionSpec::Range(start, end));
        }
        return Err("Invalid range format".to_string());
    }
    if cleaned.ends_with('+') {
        let base = cleaned.trim_end_matches('+');
        let major = base
            .parse::<u32>()
            .map_err(|_| format!("Invalid version: {base}"))?;
        return Ok(VersionSpec::Range(major, 999));
    }
    if let Ok(major) = cleaned.parse::<u32>() {
        return Ok(VersionSpec::Major(major));
    }
    Ok(VersionSpec::Exact(cleaned))
}

/// 从 Adoptium 文件名解析 `(version, tag)`。
fn parse_filename(filename: &str, major: u32) -> Option<(String, String)> {
    let key = "_hotspot_";
    let after = filename.find(key)? + key.len();
    let end = filename.find(".tar.gz").or_else(|| filename.find(".zip"))?;
    if after >= end {
        return None;
    }
    let ver = &filename[after..end];
    if major == 8 {
        // 8u492b09 → version 8u492b09, tag jdk8u492-b09
        let tag_ver = ver.replacen('b', "-b", 1);
        Some((ver.to_string(), format!("jdk{tag_ver}")))
    } else {
        // 21.0.11_10 → version 21.0.11+10, tag jdk-21.0.11+10
        let version = ver.replace('_', "+");
        Some((version.clone(), format!("jdk-{version}")))
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct CachedVersion {
    version: String,
    major: u32,
    is_lts: bool,
    tag: String,
    filename: String,
    os: String,
    arch: String,
}

#[derive(Serialize, Deserialize)]
struct VersionCache {
    fetched_at: i64,
    versions: Vec<CachedVersion>,
}

/// 清华 Adoptium 镜像目录动态发现:Java 版本来源。
pub struct AdoptiumDiscovery {
    client: Client,
    platform: Platform,
}

impl AdoptiumDiscovery {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            platform: Platform::current(),
        }
    }

    fn cache_path() -> Result<std::path::PathBuf, DiscoveryError> {
        crate::infrastructure::paths::cache_dir()
            .map(|d| d.join("java_versions.json"))
            .map_err(DiscoveryError::Io)
    }

    fn is_lts(major: u32) -> bool {
        LTS_MAJORS.contains(&major)
    }

    /// 抓单个 major 的 `/{major}/jdk/{arch}/{os}/` 目录 → filenames → 解析版本。
    async fn fetch_major(&self, major: u32) -> Result<Vec<CachedVersion>, DiscoveryError> {
        let url = format!(
            "{MIRROR_BASE}/{major}/jdk/{}/{}/",
            self.platform.arch, self.platform.os
        );
        let html = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?
            .text()
            .await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;

        let prefix = "href=\"";
        let mut out = Vec::new();
        let mut rest = html.as_str();
        while let Some(pos) = rest.find(prefix) {
            rest = &rest[pos + prefix.len()..];
            let end = match rest.find('"') {
                Some(e) => e,
                None => break,
            };
            let name = &rest[..end];
            rest = &rest[end..];
            if !name.starts_with("OpenJDK")
                || !(name.ends_with(".tar.gz") || name.ends_with(".zip"))
            {
                continue;
            }
            if let Some((version, tag)) = parse_filename(name, major) {
                out.push(CachedVersion {
                    version,
                    major,
                    is_lts: Self::is_lts(major),
                    tag,
                    filename: name.to_string(),
                    os: self.platform.os.clone(),
                    arch: self.platform.arch.clone(),
                });
            }
        }
        Ok(out)
    }

    /// 抓所有支持 major → 合并 + 写缓存。全部 major 失败则回退嵌入表。
    async fn fetch_and_cache(&self) -> Result<Vec<CachedVersion>, DiscoveryError> {
        let mut all = Vec::new();
        for &major in SUPPORTED_MAJORS {
            if let Ok(v) = self.fetch_major(major).await {
                all.extend(v);
            }
        }
        if all.is_empty() {
            return Self::embedded_versions(&self.platform);
        }
        all.sort_by(|a, b| b.major.cmp(&a.major).then(b.version.cmp(&a.version)));
        if let Ok(path) = Self::cache_path() {
            let cache = VersionCache {
                fetched_at: chrono::Utc::now().timestamp(),
                versions: all.clone(),
            };
            if let Ok(json) = serde_json::to_string(&cache) {
                let _ = std::fs::write(&path, json);
            }
        }
        Ok(all)
    }

    /// TTL 内用缓存,否则抓取;抓取失败(离线)回退嵌入表。
    async fn load_versions(&self) -> Result<Vec<CachedVersion>, DiscoveryError> {
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
            Err(_) => Self::embedded_versions(&self.platform),
        }
    }

    /// 编译期嵌入兜底:解析 `config/java_versions.toml`(RegistryEntry 格式)。
    fn embedded_versions(platform: &Platform) -> Result<Vec<CachedVersion>, DiscoveryError> {
        const EMBEDDED: &str = include_str!("../../../config/java_versions.toml");
        let parsed: toml::Value =
            toml::from_str(EMBEDDED).map_err(|e| DiscoveryError::Parse(e.to_string()))?;
        let versions = parsed
            .get("versions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| DiscoveryError::Parse("no versions in java_versions.toml".into()))?;
        let plat_key = format!("{}-{}", platform.os, platform.arch);
        let mut out = Vec::new();
        for v in versions {
            let (Some(version), Some(major)) = (
                v.get("version").and_then(|x| x.as_str()),
                v.get("major").and_then(|x| x.as_integer()),
            ) else {
                continue;
            };
            let tag = v
                .get("tag_name")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string();
            let Some(filename) = v
                .get("assets")
                .and_then(|a| a.as_table())
                .and_then(|a| a.get(&plat_key))
                .and_then(|f| f.as_str())
            else {
                continue;
            };
            out.push(CachedVersion {
                version: version.to_string(),
                major: major as u32,
                is_lts: Self::is_lts(major as u32),
                tag,
                filename: filename.to_string(),
                os: platform.os.clone(),
                arch: platform.arch.clone(),
            });
        }
        Ok(out)
    }

    fn make_resolved(cv: &CachedVersion) -> ResolvedVersion {
        ResolvedVersion {
            version: cv.version.clone(),
            major: Some(cv.major),
            is_lts: cv.is_lts,
            display: format!("Eclipse Temurin JDK {}", cv.version),
            template_vars: TemplateVars {
                version: cv.version.clone(),
                major: Some(cv.major),
                tag: Some(cv.tag.clone()),
                filename: cv.filename.clone(),
                os: cv.os.clone(),
                arch: cv.arch.clone(),
                ..Default::default()
            },
        }
    }
}

impl Default for AdoptiumDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionDiscovery for AdoptiumDiscovery {
    fn list(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<ResolvedVersion>, DiscoveryError>> + Send + '_>>
    {
        Box::pin(async {
            let versions = self.load_versions().await?;
            Ok(versions.iter().map(Self::make_resolved).collect())
        })
    }

    fn find(
        &self,
        spec: &str,
    ) -> Pin<Box<dyn Future<Output = Result<ResolvedVersion, DiscoveryError>> + Send + '_>> {
        let s = spec.to_string();
        Box::pin(async move {
            let versions = self.load_versions().await?;
            let vspec = parse_version_spec(&s).map_err(DiscoveryError::Parse)?;
            let mut matching: Vec<&CachedVersion> = match vspec {
                VersionSpec::Latest => versions.iter().collect(),
                VersionSpec::LatestLts => versions.iter().filter(|v| v.is_lts).collect(),
                VersionSpec::Major(m) => versions.iter().filter(|v| v.major == m).collect(),
                VersionSpec::Range(lo, hi) => versions
                    .iter()
                    .filter(|v| v.major >= lo && v.major <= hi)
                    .collect(),
                VersionSpec::Exact(e) => versions.iter().filter(|v| v.version == e).collect(),
            };
            matching.sort_by(|a, b| b.major.cmp(&a.major).then(b.version.cmp(&a.version)));
            matching
                .first()
                .map(|v| Self::make_resolved(v))
                .ok_or(DiscoveryError::NotFound(s))
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
    fn parse_filename_major21() {
        let (v, t) =
            parse_filename("OpenJDK21U-jdk_x64_linux_hotspot_21.0.11_10.tar.gz", 21).unwrap();
        assert_eq!(v, "21.0.11+10");
        assert_eq!(t, "jdk-21.0.11+10");
    }

    #[test]
    fn parse_filename_major17() {
        let (v, _) =
            parse_filename("OpenJDK17U-jdk_x64_linux_hotspot_17.0.19_10.tar.gz", 17).unwrap();
        assert_eq!(v, "17.0.19+10");
    }

    #[test]
    fn parse_filename_major8() {
        let (v, t) = parse_filename("OpenJDK8U-jdk_x64_linux_hotspot_8u492b09.tar.gz", 8).unwrap();
        assert_eq!(v, "8u492b09");
        assert_eq!(t, "jdk8u492-b09");
    }

    #[test]
    fn parse_filename_zip() {
        let (v, _) =
            parse_filename("OpenJDK21U-jdk_x64_windows_hotspot_21.0.11_10.zip", 21).unwrap();
        assert_eq!(v, "21.0.11+10");
    }

    #[test]
    fn parse_spec_variants() {
        assert_eq!(parse_version_spec("lts").unwrap(), VersionSpec::LatestLts);
        assert_eq!(parse_version_spec("latest").unwrap(), VersionSpec::Latest);
        assert_eq!(parse_version_spec("21").unwrap(), VersionSpec::Major(21));
        assert_eq!(
            parse_version_spec("17+").unwrap(),
            VersionSpec::Range(17, 999)
        );
        assert_eq!(
            parse_version_spec("8-11").unwrap(),
            VersionSpec::Range(8, 11)
        );
        assert!(matches!(
            parse_version_spec("21.0.1").unwrap(),
            VersionSpec::Exact(_)
        ));
    }

    #[test]
    fn embedded_versions_loads() {
        let platform = Platform::current();
        let v = AdoptiumDiscovery::embedded_versions(&platform).expect("embedded");
        assert!(!v.is_empty());
        assert!(v.iter().any(|x| x.major == 21));
    }
}
