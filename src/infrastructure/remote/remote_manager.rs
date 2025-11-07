use reqwest;
use serde::{Deserialize, Serialize};
use crate::environments::java::VersionManager;

/// Java 版本信息
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
    pub fn new(version: &str, major: u32, minor: u32, patch: u32, release_name: &str, download_url: Option<String>) -> Self {
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

/// Maven 工件信息
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
    /// 版本管理器
    version_manager: VersionManager,
}

/// Adoptium API 返回的 Java 版本信息
#[derive(Debug, Deserialize)]
pub struct AdoptiumRelease {
    pub release_name: String,
    pub version: Option<AdoptiumVersion>,
    pub binaries: Vec<AdoptiumBinary>,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumVersion {
    pub semver: String,
    pub major: u32,
    pub minor: u32,
    pub security: u32,
    pub build: Option<u32>,
    pub optional: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumBinary {
    pub os: String,
    pub architecture: String,
    pub image_type: String,
    pub package: Option<AdoptiumPackage>,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumPackage {
    pub name: String,
    pub link: String,
}

/// Adoptium 可用版本响应
#[derive(Debug, Deserialize)]
pub struct AdoptiumAvailableResponse {
    pub available_releases: Vec<u32>,
    pub available_lts_releases: Vec<u32>,
    pub most_recent_feature_release: u32,
    pub most_recent_feature_version: u32,
    pub most_recent_lts: u32,
    pub tip_version: u32,
}

/// Maven 搜索结果
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
    pub g: String,  // groupId
    pub a: String,  // artifactId
    pub latest_version: String,
    pub p: String,  // packaging
    pub timestamp: Option<u64>,
}

impl RemoteManager {
    /// 创建新的远程管理器
    pub fn new() -> Self {
        Self {
            version_manager: VersionManager::new("https://api.adoptium.net/v3"),
        }
    }

    /// 获取版本管理器的可变引用
    pub fn version_manager_mut(&mut self) -> &mut VersionManager {
        &mut self.version_manager
    }
    /// 查询可用的 Java 版本（使用新的版本管理器）
    pub async fn list_java_versions(
        &mut self,
        repo_url: Option<&str>,
        feature_version: Option<u32>,
        _os: Option<&str>,
        _arch: Option<&str>,
    ) -> Result<Vec<JavaVersionInfo>, String> {
        println!("正在查询 Java 版本信息...");

        // 优先使用新的版本管理器
        match self.version_manager.get_versions().await {
            Ok(versions) => {
                let mut result = Vec::new();

                if let Some(major) = feature_version {
                    // 返回指定主版本的 Java
                    for version in versions.iter().filter(|v| v.major == major) {
                        let version_info = JavaVersionInfo {
                            version: version.version.clone(),
                            major: Some(version.major),
                            minor: version.minor,
                            patch: version.patch,
                            release_name: version.release_name.clone(),
                            download_url: self.get_download_url_for_version(&version).await?,
                        };
                        result.push(version_info);
                    }

                    if result.is_empty() {
                        return Err(format!("未找到 Java {} 的可用版本", major));
                    }
                } else {
                    // 返回所有版本
                    for version in versions {
                        let version_info = JavaVersionInfo {
                            version: version.version.clone(),
                            major: Some(version.major),
                            minor: version.minor,
                            patch: version.patch,
                            release_name: version.release_name.clone(),
                            download_url: self.get_download_url_for_version(&version).await?,
                        };
                        result.push(version_info);
                    }
                }

                Ok(result)
            }
            Err(e) => {
                // 回退到原有的实现
                println!("⚠️  新版本管理器失败，回退到原有实现: {}", e);
                let repo_url = repo_url.unwrap_or("https://api.adoptium.net/v3");
                self.list_java_versions_legacy(repo_url, feature_version).await
            }
        }
    }

    /// 获取版本的下载 URL
    async fn get_download_url_for_version(&self, version: &crate::environments::java::JavaVersion) -> Result<Option<String>, String> {
        // 这里可以根据版本信息生成下载 URL
        // 暂时返回 None，让调用者使用默认逻辑
        Ok(None)
    }

    /// 原有的版本查询实现（作为回退）
    async fn list_java_versions_legacy(
        &self,
        repo_url: &str,
        feature_version: Option<u32>,
    ) -> Result<Vec<JavaVersionInfo>, String> {
        // 判断源的类型
        if repo_url.contains("aliyun.com") || repo_url.contains("mirrors.aliyun.com") {
            // 使用阿里云镜像逻辑
            return Self::list_java_versions_from_aliyun(feature_version).await;
        } else if repo_url.contains("adoptium") || repo_url.contains("adoptopenjdk") {
            // 使用 Adoptium API 逻辑
            return Self::list_java_versions_from_adoptium(repo_url, feature_version).await;
        } else {
            // 默认使用 Adoptium API
            return Self::list_java_versions_from_adoptium(repo_url, feature_version).await;
        }
    }

    /// 从阿里云镜像查询 Java 版本
    async fn list_java_versions_from_aliyun(feature_version: Option<u32>) -> Result<Vec<JavaVersionInfo>, String> {
        let mirror_base = "https://mirrors.aliyun.com/eclipse/temurin-compliance/temurin";
        let mut versions = Vec::new();

        if let Some(major_version) = feature_version {
            // 查询特定主要版本
            match major_version {
                8 => {
                    versions.push(JavaVersionInfo::new(
                        "8.0.392",
                        8, 0, 392,
                        "jdk-8u392",
                        Some(format!("{}/8/jdk8u392-b08/{}",
                            mirror_base, Self::get_download_filename(8, "8.0.392", "x64_windows"))),
                    ));
                    versions.push(JavaVersionInfo {
                        version: "8.0.382".to_string(),
                        major: Some(8),
                        minor: Some(0),
                        patch: Some(382),
                        release_name: "jdk-8u382".to_string(),
                        download_url: Some(format!("{}/8/jdk8u382-b05/{}",
                            mirror_base, Self::get_download_filename(8, "8.0.382", "x64_windows"))),
                    });
                }
                11 => {
                    versions.push(JavaVersionInfo {
                        version: "11.0.23".to_string(),
                        major: Some(11),
                        minor: Some(0),
                        patch: Some(23),
                        release_name: "jdk-11.0.23+9".to_string(),
                        download_url: Some(format!("{}/11/jdk-11.0.23+9/{}",
                            mirror_base, Self::get_download_filename(11, "11.0.23", "x64_windows"))),
                    });
                    versions.push(JavaVersionInfo {
                        version: "11.0.22".to_string(),
                        major: Some(11),
                        minor: Some(0),
                        patch: Some(22),
                        release_name: "jdk-11.0.22+7".to_string(),
                        download_url: Some(format!("{}/11/jdk-11.0.22+7/{}",
                            mirror_base, Self::get_download_filename(11, "11.0.22", "x64_windows"))),
                    });
                }
                17 => {
                    versions.push(JavaVersionInfo {
                        version: "17.0.12".to_string(),
                        major: Some(17),
                        minor: Some(0),
                        patch: Some(12),
                        release_name: "jdk-17.0.12+7".to_string(),
                        download_url: Some(format!("{}/17/jdk-17.0.12+7/{}",
                            mirror_base, Self::get_download_filename(17, "17.0.12", "x64_windows"))),
                    });
                    versions.push(JavaVersionInfo {
                        version: "17.0.11".to_string(),
                        major: Some(17),
                        minor: Some(0),
                        patch: Some(11),
                        release_name: "jdk-17.0.11+9".to_string(),
                        download_url: Some(format!("{}/17/jdk-17.0.11+9/{}",
                            mirror_base, Self::get_download_filename(17, "17.0.11", "x64_windows"))),
                    });
                }
                21 => {
                    versions.push(JavaVersionInfo::new(
                        "21.0.4",
                        21, 0, 4,
                        "jdk-21.0.4+7",
                        Some(format!("{}/21/jdk-21.0.4+7/{}",
                            mirror_base, Self::get_download_filename(21, "21.0.4", "x64_windows"))),
                    ));
                    versions.push(JavaVersionInfo::new(
                        "21.0.3",
                        21, 0, 3,
                        "jdk-21.0.3+9",
                        Some(format!("{}/21/jdk-21.0.3+9/{}",
                            mirror_base, Self::get_download_filename(21, "21.0.3", "x64_windows"))),
                    ));
                }
                _ => {
                    return Err(format!("不支持的 Java 版本: {}. 支持的版本: 8, 11, 17, 21", major_version));
                }
            }
        } else {
            // 查询所有可用的 LTS 版本
            versions.push(JavaVersionInfo::new(
                "21.0.4",
                21, 0, 4,
                "OpenJDK 21 (Latest LTS)",
                Some(format!("{}/21/jdk-21.0.4+7/{}",
                    mirror_base, Self::get_download_filename(21, "21.0.4", "x64_windows"))),
            ));
            versions.push(JavaVersionInfo::new(
                "17.0.12",
                17, 0, 12,
                "OpenJDK 17 (LTS)",
                Some(format!("{}/17/jdk-17.0.12+7/{}",
                    mirror_base, Self::get_download_filename(17, "17.0.12", "x64_windows"))),
            ));
            versions.push(JavaVersionInfo::new(
                "11.0.23",
                11, 0, 23,
                "OpenJDK 11 (LTS)",
                Some(format!("{}/11/jdk-11.0.23+9/{}",
                    mirror_base, Self::get_download_filename(11, "11.0.23", "x64_windows"))),
            ));
            versions.push(JavaVersionInfo {
                version: "8.0.392".to_string(),
                major: Some(8),
                minor: Some(0),
                patch: Some(392),
                release_name: "OpenJDK 8 (LTS)".to_string(),
                download_url: Some(format!("{}/8/jdk8u392-b08/{}",
                    mirror_base, Self::get_download_filename(8, "8.0.392", "x64_windows"))),
            });
        }

        Ok(versions)
    }

    /// 从 Adoptium/GitHub 查询 Java 版本（简化版本，使用 GitHub 下载链接）
    async fn list_java_versions_from_adoptium(
        repo_url: &str,
        feature_version: Option<u32>,
    ) -> Result<Vec<JavaVersionInfo>, String> {
        println!("使用 GitHub/Adoptium 源查询 Java 版本...");

        // 获取平台信息
        let (platform, arch, os) = Self::detect_platform_info();

        let mut versions = Vec::new();

        if let Some(major_version) = feature_version {
            // 查询特定主要版本
            match major_version {
                8 => {
                    versions.push(JavaVersionInfo {
                        version: "8.0.422".to_string(),
                        major: Some(8),
                        minor: Some(0),
                        patch: Some(422),
                        release_name: "OpenJDK 8.0.422".to_string(),
                        download_url: Some(format!("https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u422-b05/OpenJDK8U-jdk_{}_{}_hotspot_8u422b05.{}",
                            arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
                    });
                }
                11 => {
                    versions.push(JavaVersionInfo {
                        version: "11.0.24".to_string(),
                        major: Some(11),
                        minor: Some(0),
                        patch: Some(24),
                        release_name: "OpenJDK 11.0.24".to_string(),
                        download_url: Some(format!("https://github.com/adoptium/temurin11-binaries/releases/download/jdk-11.0.24%2B8/OpenJDK11U-jdk_{}_{}_hotspot_11.0.24_8.{}",
                            arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
                    });
                }
                17 => {
                    versions.push(JavaVersionInfo {
                        version: "17.0.12".to_string(),
                        major: Some(17),
                        minor: Some(0),
                        patch: Some(12),
                        release_name: "OpenJDK 17.0.12".to_string(),
                        download_url: Some(format!("https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.12%2B7/OpenJDK17U-jdk_{}_{}_hotspot_17.0.12_7.{}",
                            arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
                    });
                }
                21 => {
                    versions.push(JavaVersionInfo {
                        version: "21.0.4".to_string(),
                        major: Some(21),
                        minor: Some(0),
                        patch: Some(4),
                        release_name: "OpenJDK 21.0.4".to_string(),
                        download_url: Some(format!("https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.4%2B7/OpenJDK21U-jdk_{}_{}_hotspot_21.0.4_7.{}",
                            arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
                    });
                }
                _ => {
                    return Err(format!("不支持的 Java 版本: {}. 支持的版本: 8, 11, 17, 21", major_version));
                }
            }
        } else {
            // 查询所有可用的 LTS 版本
            versions.push(JavaVersionInfo {
                version: "21.0.4".to_string(),
                major: Some(21),
                minor: Some(0),
                patch: Some(4),
                release_name: "OpenJDK 21 (Latest LTS)".to_string(),
                download_url: Some(format!("https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.4%2B7/OpenJDK21U-jdk_{}_{}_hotspot_21.0.4_7.{}",
                    arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
            });
            versions.push(JavaVersionInfo {
                version: "17.0.12".to_string(),
                major: Some(17),
                minor: Some(0),
                patch: Some(12),
                release_name: "OpenJDK 17 (LTS)".to_string(),
                download_url: Some(format!("https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.12%2B7/OpenJDK17U-jdk_{}_{}_hotspot_17.0.12_7.{}",
                    arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
            });
            versions.push(JavaVersionInfo {
                version: "11.0.24".to_string(),
                major: Some(11),
                minor: Some(0),
                patch: Some(24),
                release_name: "OpenJDK 11 (LTS)".to_string(),
                download_url: Some(format!("https://github.com/adoptium/temurin11-binaries/releases/download/jdk-11.0.24%2B8/OpenJDK11U-jdk_{}_{}_hotspot_11.0.24_8.{}",
                    arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
            });
            versions.push(JavaVersionInfo {
                version: "8.0.422".to_string(),
                major: Some(8),
                minor: Some(0),
                patch: Some(422),
                release_name: "OpenJDK 8 (LTS)".to_string(),
                download_url: Some(format!("https://github.com/adoptium/temurin8-binaries/releases/download/jdk8u422-b05/OpenJDK8U-jdk_{}_{}_hotspot_8u422b05.{}",
                    arch, os, if cfg!(target_os = "windows") { "zip" } else { "tar.gz" })),
            });
        }

        Ok(versions)
    }

    /// 获取下载文件名
    fn get_download_filename(major_version: u32, version: &str, platform: &str) -> String {
        // 根据当前平台选择合适的文件扩展名
        let extension = if cfg!(target_os = "windows") {
            ".zip"
        } else {
            ".tar.gz"
        };

        // 阿里云镜像的文件命名格式
        match major_version {
            8 => {
                if version == "8.0.392" {
                    format!("OpenJDK8U-jdk_{}_hotspot_8u392b08{}", platform, extension)
                } else if version == "8.0.382" {
                    format!("OpenJDK8U-jdk_{}_hotspot_8u382b05{}", platform, extension)
                } else {
                    format!("OpenJDK8U-jdk_{}_hotspot_{}{}", platform, version, extension)
                }
            }
            11 => {
                if version == "11.0.23" {
                    format!("OpenJDK11U-jdk_{}_hotspot_11.0.23_9{}", platform, extension)
                } else if version == "11.0.22" {
                    format!("OpenJDK11U-jdk_{}_hotspot_11.0.22_7{}", platform, extension)
                } else {
                    format!("OpenJDK11U-jdk_{}_hotspot_{}{}", platform, version, extension)
                }
            }
            17 => {
                if version == "17.0.12" {
                    format!("OpenJDK17U-jdk_{}_hotspot_17.0.12_7{}", platform, extension)
                } else if version == "17.0.11" {
                    format!("OpenJDK17U-jdk_{}_hotspot_17.0.11_9{}", platform, extension)
                } else {
                    format!("OpenJDK17U-jdk_{}_hotspot_{}{}", platform, version, extension)
                }
            }
            21 => {
                if version == "21.0.4" {
                    format!("OpenJDK21U-jdk_{}_hotspot_21.0.4_7{}", platform, extension)
                } else if version == "21.0.3" {
                    format!("OpenJDK21U-jdk_{}_hotspot_21.0.3_9{}", platform, extension)
                } else {
                    format!("OpenJDK21U-jdk_{}_hotspot_{}{}", platform, version, extension)
                }
            }
            _ => {
                format!("OpenJDK{}U-jdk_{}_hotspot_{}{}", major_version, platform, version, extension)
            }
        }
    }

    /// 检测平台信息
    fn detect_platform_info() -> (String, String, String) {
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "aarch64",
            "x86" => "x86",
            _ => "x64",
        };

        let os = match std::env::consts::OS {
            "windows" => "windows",
            "macos" => "mac",
            "linux" => "linux",
            _ => "linux",
        };

        let platform = format!("{}_{}", arch, os);

        (platform, arch.to_string(), os.to_string())
    }

    /// 查找适合当前平台的下载链接
    fn find_download_url(binaries: &[AdoptiumBinary]) -> Option<String> {
        let current_os = std::env::consts::OS;
        let current_arch = std::env::consts::ARCH;

        for binary in binaries {
            let os_match = match current_os {
                "windows" => binary.os == "windows",
                "macos" => binary.os == "mac",
                "linux" => binary.os == "linux",
                _ => false,
            };

            let arch_match = match current_arch {
                "x86_64" => binary.architecture == "x64",
                "aarch64" => binary.architecture == "aarch64",
                "x86" => binary.architecture == "x86",
                _ => false,
            };

            if os_match && arch_match && binary.image_type == "jdk" {
                if let Some(package) = &binary.package {
                    return Some(package.link.clone());
                }
            }
        }

        None
    }

    /// 查询 Maven 依赖的可用版本
    pub async fn list_maven_versions(
        repo_url: &str,
        group_id: &str,
        artifact_id: &str,
    ) -> Result<Vec<MavenVersionInfo>, String> {
        let client = reqwest::Client::new();

        // 构建搜索 URL
        let query = format!("g:{} AND a:{}", group_id, artifact_id);
        let url = format!("?q={}&rows=100&wt=json", urlencoding::encode(&query));
        let full_url = if repo_url.contains("/solrsearch/select") {
            format!("{}{}", repo_url, &url[1..]) // 移除开头的 ?
        } else {
            format!("{}/solrsearch/select{}", repo_url, url)
        };

        println!("正在查询 Maven 依赖: {}", full_url);

        let response = client
            .get(&full_url)
            .header("User-Agent", "fnva/0.0.4")
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API 请求失败: {}", response.status()));
        }

        let search_result: MavenSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

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

        // 按时间戳排序（最新的在前）
        versions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(versions)
    }

    /// 搜索 Maven 工件
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
            format!("{}/solrsearch/select?{}", repo_url, search_query)
        };

        println!("正在搜索 Maven 工件: {}", full_url);

        let response = client
            .get(&full_url)
            .header("User-Agent", "fnva/0.0.4")
            .send()
            .await
            .map_err(|e| format!("请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API 请求失败: {}", response.status()));
        }

        let search_result: MavenSearchResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let mut artifacts = Vec::new();

        for artifact in search_result.response.docs {
            let group_id = artifact.g.clone();
            let artifact_id = artifact.a.clone();
            let description = format!("{}:{}", group_id, artifact_id);
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


// 添加 urlencoding 依赖
use urlencoding;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_java_versions() {
        let versions = RemoteManager::list_java_versions(
            "https://api.adoptium.net/v3",
            Some(17),
            None,
            None,
        ).await;

        assert!(versions.is_ok());
        let versions = versions.unwrap();
        assert!(!versions.is_empty());

        for version in versions.iter().take(5) {
            println!("Java {}: {}", version.major.unwrap_or(0), version.version);
        }
    }

    #[tokio::test]
    async fn test_list_maven_versions() {
        let versions = RemoteManager::list_maven_versions(
            "https://search.maven.org/solrsearch/select",
            "org.springframework.boot",
            "spring-boot-starter",
        ).await;

        assert!(versions.is_ok());
        let versions = versions.unwrap();
        assert!(!versions.is_empty());

        for version in versions.iter().take(5) {
            println!("{}:{}:{}",
                version.group_id.as_deref().unwrap_or("unknown"),
                version.artifact_id.as_deref().unwrap_or("unknown"),
                version.version);
        }
    }
}