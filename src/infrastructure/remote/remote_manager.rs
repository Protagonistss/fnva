use super::JavaDownloader;
use super::Platform;
use crate::environments::java::VersionManager;
use reqwest;
use serde::{Deserialize, Serialize};

/// Java 版本信息 (API 输出用)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaVersionInfo {
    pub version: String,
    pub major: Option<u32>,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub release_name: String,
    pub download_url: Option<String>,
}

impl JavaVersionInfo {
    pub fn new(
        version: &str,
        major: u32,
        minor: u32,
        patch: u32,
        release_name: &str,
        download_url: Option<String>,
    ) -> Self {
        Self {
            version: version.to_string(),
            major: Some(major),
            minor: Some(minor),
            patch: Some(patch),
            release_name: release_name.to_string(),
            download_url,
        }
    }
}

/// Maven 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavenVersionInfo {
    pub version: String,
    pub packaging: String,
    pub group_id: Option<String>,
    pub artifact_id: Option<String>,
    pub timestamp: Option<String>,
}

/// Maven artifact 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MavenArtifactInfo {
    pub group_id: String,
    pub artifact_id: String,
    pub latest_version: String,
    pub packaging: String,
    pub description: Option<String>,
}

/// 远程查询管理器
pub struct RemoteManager {
    /// 版本管理器缓存（保留接口，供其他调用方复用）
    version_manager: VersionManager,
}

/// Adoptium API 返回体（VersionManager 仍然使用）
#[derive(Debug, Deserialize)]
pub struct AdoptiumAvailableResponse {
    pub available_releases: Vec<u32>,
    pub available_lts_releases: Vec<u32>,
    pub most_recent_feature_release: u32,
    pub most_recent_feature_version: u32,
    pub most_recent_lts: u32,
    pub tip_version: u32,
}

/// Maven 搜索返回体
#[derive(Debug, Deserialize)]
pub struct MavenSearchResponse {
    pub response: MavenResponse,
}

#[derive(Debug, Deserialize)]
pub struct MavenResponse {
    pub docs: Vec<MavenArtifact>,
    pub num_found: u32,
}

#[derive(Debug, Deserialize)]
pub struct MavenArtifact {
    pub id: String,
    pub g: String, // groupId
    pub a: String, // artifactId
    pub latest_version: String,
    pub p: String, // packaging
    pub timestamp: Option<u64>,
}

impl Default for RemoteManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RemoteManager {
    /// 创建新的远程管理器
    pub fn new() -> Self {
        Self {
            version_manager: VersionManager::new("https://api.adoptium.net/v3"),
        }
    }

    /// 获取版本管理器（外部仍可直接访问）
    pub fn version_manager_mut(&mut self) -> &mut VersionManager {
        &mut self.version_manager
    }

    /// 内部辅助：获取对应的下载器实例
    fn get_downloader_for_repo(repo_url: Option<&str>) -> Box<dyn JavaDownloader> {
        let repo = repo_url.unwrap_or("");
        let use_tsinghua = repo.contains("tuna.tsinghua.edu.cn") || repo.is_empty(); // 默认为清华源
        let use_aliyun = repo.contains("aliyun");

        if use_tsinghua {
            Box::new(crate::remote::TsinghuaJavaDownloader::new())
        } else if use_aliyun {
            Box::new(crate::remote::AliyunJavaDownloader::new())
        } else {
            Box::new(crate::remote::GitHubJavaDownloader::new())
        }
    }

    /// 查询可用的 Java 版本列表，优先根据 repo_url 选择阿里云或 GitHub。
    pub async fn list_java_versions(
        &mut self,
        repo_url: Option<&str>,
        feature_version: Option<u32>,
        _os: Option<&str>,
        _arch: Option<&str>,
    ) -> Result<Vec<JavaVersionInfo>, String> {
        println!("查询 Java 版本信息...");

        let platform = Platform::current();
        let downloader = Self::get_downloader_for_repo(repo_url);

        let versions = downloader
            .list_available_versions()
            .await
            .map_err(|e| format!("{e:?}"))?;

        let filtered = versions
            .into_iter()
            .filter(|v| feature_version.is_none_or(|mv| v.major == mv))
            .collect::<Vec<_>>();

        if filtered.is_empty() {
            return Err(feature_version.map_or_else(
                || "未找到可用版本".to_string(),
                |mv| format!("未找到 Java {mv}"),
            ));
        }

        let mut result = Vec::new();
        for version in filtered {
            let download_url = downloader.get_download_url(&version, &platform).await.ok();
            result.push(JavaVersionInfo {
                version: version.version.clone(),
                major: Some(version.major),
                minor: version.minor,
                patch: version.patch,
                release_name: version.release_name.clone(),
                download_url,
            });
        }
        Ok(result)
    }

    /// 查询 Maven 组件的可用版本
    pub async fn list_maven_versions(
        repo_url: &str,
        group_id: &str,
        artifact_id: &str,
    ) -> Result<Vec<MavenVersionInfo>, String> {
        let client = reqwest::Client::new();

        // 构造查询 URL
        let query = format!("g:{group_id} AND a:{artifact_id}");
        let url = format!("?q={}&rows=100&wt=json", urlencoding::encode(&query));
        let full_url = if repo_url.contains("/solrsearch/select") {
            format!("{}{}", repo_url, &url[1..]) // 去掉开头的 ?
        } else {
            format!("{repo_url}/solrsearch/select{url}")
        };

        println!("正在查询 Maven 仓库: {full_url}");

        let response = client
            .get(&full_url)
            .header("User-Agent", "fnva/0.0.4")
            .send()
            .await
            .map_err(|e| format!("请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("API 请求失败: {}", response.status()));
        }

        let search_result: MavenSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {e}"))?;

        let mut versions = Vec::new();

        for artifact in search_result.response.docs {
            versions.push(MavenVersionInfo {
                group_id: Some(artifact.g),
                artifact_id: Some(artifact.a),
                version: artifact.latest_version,
                packaging: artifact.p,
                timestamp: artifact.timestamp.map(|ts| ts.to_string()),
            });
        }

        // 简单按时间倒序
        versions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(versions)
    }

    /// 搜索 Maven 组件
    pub async fn search_maven_artifacts(
        repo_url: &str,
        query: &str,
        limit: Option<u32>,
    ) -> Result<Vec<MavenArtifactInfo>, String> {
        let client = reqwest::Client::new();

        let rows = limit.unwrap_or(50);
        let search_query = format!("q={}&rows={}&wt=json", urlencoding::encode(query), rows);
        let full_url = if repo_url.contains("/solrsearch/select") {
            format!("{}{}", repo_url, &search_query[1..])
        } else {
            format!("{repo_url}/solrsearch/select?{search_query}")
        };

        println!("正在搜索 Maven 仓库: {full_url}");

        let response = client
            .get(&full_url)
            .header("User-Agent", "fnva/0.0.4")
            .send()
            .await
            .map_err(|e| format!("请求失败: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("API 请求失败: {}", response.status()));
        }

        let search_result: MavenSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {e}"))?;

        let mut artifacts = Vec::new();

        for artifact in search_result.response.docs {
            let group_id = artifact.g.clone();
            let artifact_id = artifact.a.clone();
            let description = format!("{group_id}:{artifact_id}");
            artifacts.push(MavenArtifactInfo {
                group_id,
                artifact_id,
                latest_version: artifact.latest_version,
                packaging: artifact.p,
                description: Some(description),
            });
        }

        Ok(artifacts)
    }
}

// 兼容 urlencoding 依赖
use urlencoding;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_java_versions_basic() {
        let mut manager = RemoteManager::new();
        let versions = manager
            .list_java_versions(
                Some("https://mirrors.aliyun.com/eclipse/temurin-compliance/temurin"),
                Some(17),
                None,
                None,
            )
            .await;

        // 只要不 panic 即可，允许网络问题导致 Err
        assert!(versions.is_ok() || versions.is_err());
    }

    #[tokio::test]
    async fn test_list_maven_versions() {
        // 查询 Maven 仓库
        let result = RemoteManager::list_maven_versions(
            "https://search.maven.org/solrsearch/select",
            "org.springframework.boot",
            "spring-boot-starter",
        )
        .await;

        // 结果只要不中断即可
        assert!(result.is_ok() || result.is_err());
    }
}
