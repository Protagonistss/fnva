use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Adoptium API 可用版本响应
#[derive(Debug, Clone, Deserialize)]
pub struct AdoptiumAvailableResponse {
    pub available_releases: Vec<u32>,
    pub available_lts_releases: Vec<u32>,
    pub most_recent_lts: u32,
    pub most_recent_feature_version: u32,
}

/// Java 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersion {
    pub version: String,
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub release_name: String,
    pub semver: String,
    pub is_lts: bool,
    pub is_latest: bool,
}

impl JavaVersion {
    pub fn new(version: String, major: u32, semver: String, is_lts: bool) -> Self {
        Self {
            version: version.clone(),
            major,
            minor: None,
            patch: None,
            release_name: format!("OpenJDK {}{}", major, if is_lts { " (LTS)" } else { "" }),
            semver,
            is_lts,
            is_latest: false,
        }
    }

    /// 解析版本字符串
    pub fn from_semver(semver: &str, is_lts: bool) -> Result<Self, String> {
        // 解析 semver 格式，如 "21.0.4+7"
        let parts: Vec<&str> = semver.split('+').collect();
        let version_part = parts[0];

        let version_parts: Vec<&str> = version_part.split('.').collect();
        if version_parts.len() < 2 {
            return Err(format!("无效的版本格式: {semver}"));
        }

        let major = version_parts[0]
            .parse::<u32>()
            .map_err(|_| format!("无效的主版本号: {}", version_parts[0]))?;

        let minor = version_parts.get(1).and_then(|s| s.parse::<u32>().ok());
        let patch = version_parts.get(2).and_then(|s| s.parse::<u32>().ok());

        Ok(Self {
            version: version_part.to_string(),
            major,
            minor,
            patch,
            release_name: format!("OpenJDK {}{}", major, if is_lts { " (LTS)" } else { "" }),
            semver: semver.to_string(),
            is_lts,
            is_latest: false,
        })
    }
}

/// 版本解析结果
#[derive(Debug, Clone, PartialEq)]
pub enum VersionSpec {
    Major(u32),
    Exact(String),
    LatestLts,
    Latest,
    Range(u32, u32), // 起始版本，结束版本
}

/// 版本管理器
pub struct VersionManager {
    /// 版本缓存
    version_cache: Option<VersionCache>,
    /// Adoptium API URL
    api_url: String,
}

/// 版本缓存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCache {
    pub versions: Vec<JavaVersion>,
    pub available_releases: Vec<u32>,
    pub lts_releases: Vec<u32>,
    pub most_recent_lts: u32,
    pub most_recent_feature: u32,
    pub timestamp: u64,
    pub ttl: u64, // 缓存生存时间（秒）
}

impl VersionCache {
    /// 创建新的缓存
    pub fn new(
        versions: Vec<JavaVersion>,
        available_response: AdoptiumAvailableResponse,
        ttl: u64,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            versions,
            available_releases: available_response.available_releases,
            lts_releases: available_response.available_lts_releases,
            most_recent_lts: available_response.most_recent_lts,
            most_recent_feature: available_response.most_recent_feature_version,
            timestamp,
            ttl,
        }
    }

    /// 检查缓存是否过期
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.timestamp) > self.ttl
    }

    /// 获取最新 LTS 版本
    pub fn get_latest_lts(&self) -> Option<&JavaVersion> {
        self.versions
            .iter()
            .filter(|v| v.is_lts)
            .max_by(|a, b| match (a.minor, b.minor) {
                (Some(a_min), Some(b_min)) => a_min.cmp(&b_min),
                _ => a.major.cmp(&b.major),
            })
    }

    /// 获取最新版本
    pub fn get_latest(&self) -> Option<&JavaVersion> {
        self.versions.iter().max_by(|a, b| {
            match (
                a.major.cmp(&b.major),
                a.minor.cmp(&b.minor),
                a.patch.cmp(&b.patch),
            ) {
                (
                    std::cmp::Ordering::Equal,
                    std::cmp::Ordering::Equal,
                    std::cmp::Ordering::Equal,
                ) => std::cmp::Ordering::Equal,
                (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal, patch_cmp) => patch_cmp,
                (std::cmp::Ordering::Equal, minor_cmp, _) => minor_cmp,
                (major_cmp, _, _) => major_cmp,
            }
        })
    }

    /// 根据主版本号查找版本
    pub fn find_by_major(&self, major: u32) -> Vec<&JavaVersion> {
        self.versions.iter().filter(|v| v.major == major).collect()
    }

    /// 查找精确匹配的版本
    pub fn find_exact(&self, version: &str) -> Option<&JavaVersion> {
        self.versions
            .iter()
            .find(|v| v.version == version || v.semver == version)
    }
}

impl VersionManager {
    /// 创建新的版本管理器
    pub fn new(api_url: &str) -> Self {
        Self {
            version_cache: None,
            api_url: api_url.to_string(),
        }
    }

    /// 解析版本规格
    pub fn parse_version_spec(spec: &str) -> Result<VersionSpec, String> {
        let spec_cleaned = spec.trim().to_lowercase();

        // 使用 if-let 链而不是 match 来避免借用问题
        if spec_cleaned == "lts" || spec_cleaned == "latest-lts" {
            Ok(VersionSpec::LatestLts)
        } else if spec_cleaned == "latest" || spec_cleaned == "newest" {
            Ok(VersionSpec::Latest)
        } else {
            // 处理各种版本格式
            let cleaned = spec_cleaned
                .replace("v", "")
                .replace("java", "")
                .replace("jdk", "")
                .replace("openjdk", "");

            // 检查是否是范围格式 (如 "8-11", "17+")
            if cleaned.contains('-') {
                let parts: Vec<&str> = cleaned.split('-').collect();
                if parts.len() == 2 {
                    let start = parts[0]
                        .parse::<u32>()
                        .map_err(|_| format!("无效的起始版本: {}", parts[0]))?;
                    let end = parts[1]
                        .parse::<u32>()
                        .map_err(|_| format!("无效的结束版本: {}", parts[1]))?;
                    Ok(VersionSpec::Range(start, end))
                } else {
                    Err("无效的范围格式".to_string())
                }
            } else if cleaned.ends_with('+') {
                let base_version = cleaned.trim_end_matches('+');
                let major = base_version
                    .parse::<u32>()
                    .map_err(|_| format!("无效的版本号: {base_version}"))?;
                Ok(VersionSpec::Range(major, 999)) // 999 表示无上限
            } else {
                // 尝试解析为主版本号
                if let Ok(major) = cleaned.parse::<u32>() {
                    Ok(VersionSpec::Major(major))
                } else {
                    // 作为精确版本处理
                    Ok(VersionSpec::Exact(cleaned))
                }
            }
        }
    }

    /// 获取版本信息
    pub async fn get_versions(&mut self) -> Result<Vec<JavaVersion>, String> {
        // 检查缓存
        if let Some(cache) = &self.version_cache {
            if !cache.is_expired() {
                return Ok(cache.versions.clone());
            }
        }

        // 从远程获取版本信息
        self.refresh_versions().await?;
        Ok(self.version_cache.as_ref().unwrap().versions.clone())
    }

    /// 刷新版本信息
    pub async fn refresh_versions(&mut self) -> Result<(), String> {
        println!("🔄 正在获取最新 Java 版本信息...");

        // 从 Adoptium API 获取可用版本
        let available_url = format!("{}/available_releases", self.api_url);
        let client = reqwest::Client::new();

        let available_response = client
            .get(&available_url)
            .header("User-Agent", "fnva/0.0.5")
            .send()
            .await
            .map_err(|e| format!("获取可用版本失败: {e}"))?;

        if !available_response.status().is_success() {
            return Err(format!("API 请求失败: {}", available_response.status()));
        }

        let available: AdoptiumAvailableResponse = available_response
            .json()
            .await
            .map_err(|e| format!("解析版本信息失败: {e}"))?;

        // 构建版本列表
        let mut versions = Vec::new();

        // 添加主要版本
        for &major in &available.available_releases {
            if let Ok(version_info) = self.get_version_details(major).await {
                versions.push(version_info);
            }
        }

        // 按版本号排序
        versions.sort_by(|a, b| {
            b.major
                .cmp(&a.major)
                .then(b.minor.cmp(&a.minor))
                .then(b.patch.cmp(&a.patch))
        });

        // 创建缓存（TTL 为 1 小时）
        let cache = VersionCache::new(versions, available, 3600);
        self.version_cache = Some(cache);

        println!("✅ 版本信息已更新");
        Ok(())
    }

    /// 获取特定版本的详细信息
    async fn get_version_details(&self, major: u32) -> Result<JavaVersion, String> {
        // 这里可以调用更详细的 API 来获取版本信息
        // 暂时使用基本版本信息
        let is_lts = [8, 11, 17, 21].contains(&major);
        let version = JavaVersion::new(
            format!("{major}.0.0"),
            major,
            format!("{major}.0.0+0"),
            is_lts,
        );
        Ok(version)
    }

    /// 根据规格解析版本
    pub async fn resolve_version(&mut self, spec: &VersionSpec) -> Result<JavaVersion, String> {
        let versions = self.get_versions().await?;

        match spec {
            VersionSpec::Major(major) => {
                let matches: Vec<JavaVersion> = versions
                    .iter()
                    .filter(|v| v.major == *major)
                    .cloned()
                    .collect();

                if matches.is_empty() {
                    return Err(format!("未找到 Java {major} 的可用版本"));
                }

                // 返回最新的匹配版本
                Ok(matches[0].clone())
            }
            VersionSpec::Exact(version) => {
                if let Some(found) = versions
                    .iter()
                    .find(|v| v.version == *version || v.semver == *version)
                {
                    Ok(found.clone())
                } else {
                    Err(format!("未找到版本: {version}"))
                }
            }
            VersionSpec::LatestLts => {
                if let Some(lts) =
                    versions
                        .iter()
                        .filter(|v| v.is_lts)
                        .max_by(|a, b| match (a.minor, b.minor) {
                            (Some(a_min), Some(b_min)) => a_min.cmp(&b_min),
                            _ => a.major.cmp(&b.major),
                        })
                {
                    Ok(lts.clone())
                } else {
                    Err("未找到 LTS 版本".to_string())
                }
            }
            VersionSpec::Latest => {
                if let Some(latest) = versions.iter().max_by(|a, b| {
                    match (
                        a.major.cmp(&b.major),
                        a.minor.cmp(&b.minor),
                        a.patch.cmp(&b.patch),
                    ) {
                        (std::cmp::Ordering::Equal, std::cmp::Ordering::Equal, patch_cmp) => {
                            patch_cmp
                        }
                        (std::cmp::Ordering::Equal, minor_cmp, _) => minor_cmp,
                        (major_cmp, _, _) => major_cmp,
                    }
                }) {
                    Ok(latest.clone())
                } else {
                    Err("未找到可用版本".to_string())
                }
            }
            VersionSpec::Range(start, end) => {
                let matches: Vec<JavaVersion> = versions
                    .iter()
                    .filter(|v| v.major >= *start && v.major <= *end)
                    .cloned()
                    .collect();

                if matches.is_empty() {
                    return Err(format!("未找到版本范围 {start}-{end} 的可用版本"));
                }

                // 返回范围内最新的版本
                Ok(matches[0].clone())
            }
        }
    }

    /// 推荐相近版本
    pub fn suggest_alternatives(&self, requested: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        if let Some(cache) = &self.version_cache {
            // 尝试找到相近的主版本号
            if let Ok(requested_major) = requested.parse::<u32>() {
                for available in &cache.available_releases {
                    let diff = (*available as i32 - requested_major as i32).abs();
                    if diff <= 2 && diff != 0 {
                        // 相差不超过 2 个版本
                        suggestions.push(format!("Java {available}"));
                    }
                }
            }

            // 如果是 LTS 请求，推荐最新 LTS
            if requested.to_lowercase().contains("lts") {
                suggestions.push(format!("Java {} (Latest LTS)", cache.most_recent_lts));
            }
        }

        suggestions
    }

    /// 检查版本是否可用
    pub async fn is_version_available(&mut self, version: &str) -> bool {
        if let Ok(spec) = Self::parse_version_spec(version) {
            self.resolve_version(&spec).await.is_ok()
        } else {
            false
        }
    }

    /// 获取支持的版本列表
    pub async fn list_available_versions(&mut self) -> Result<Vec<String>, String> {
        let versions = self.get_versions().await?;
        let mut result = Vec::new();

        for version in versions {
            result.push(format!(
                "Java {}{} ({})",
                version.major,
                if version.is_lts { " (LTS)" } else { "" },
                version.semver
            ));
        }

        Ok(result)
    }

    /// 清除缓存
    pub fn clear_cache(&mut self) {
        self.version_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_spec() {
        assert_eq!(
            VersionManager::parse_version_spec("21").unwrap(),
            VersionSpec::Major(21)
        );
        assert_eq!(
            VersionManager::parse_version_spec("v21").unwrap(),
            VersionSpec::Major(21)
        );
        assert_eq!(
            VersionManager::parse_version_spec("jdk21").unwrap(),
            VersionSpec::Major(21)
        );
        assert_eq!(
            VersionManager::parse_version_spec("lts").unwrap(),
            VersionSpec::LatestLts
        );
        assert_eq!(
            VersionManager::parse_version_spec("latest").unwrap(),
            VersionSpec::Latest
        );
        assert_eq!(
            VersionManager::parse_version_spec("8-11").unwrap(),
            VersionSpec::Range(8, 11)
        );
        assert_eq!(
            VersionManager::parse_version_spec("17+").unwrap(),
            VersionSpec::Range(17, 999)
        );
        assert_eq!(
            VersionManager::parse_version_spec("11.0.15").unwrap(),
            VersionSpec::Exact("11.0.15".to_string())
        );
    }

    #[test]
    fn test_java_version_from_semver() {
        let version = JavaVersion::from_semver("21.0.4+7", true).unwrap();
        assert_eq!(version.major, 21);
        assert_eq!(version.minor, Some(0));
        assert_eq!(version.patch, Some(4));
        assert!(version.is_lts);
    }
}
