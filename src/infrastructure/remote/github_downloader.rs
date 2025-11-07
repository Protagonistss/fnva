use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// GitHub Java 发行版信息（从 jdk 仓库获取）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubJavaRelease {
    pub tag_name: String,
    pub name: String,
    pub prerelease: bool,
    pub published_at: String,
    pub assets: Vec<GitHubAsset>,
    pub html_url: String,
}

/// GitHub 资源文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: String,
}

/// Java 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubJavaVersion {
    pub version: String,
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub tag_name: String,
    pub release_name: String,
    pub download_urls: HashMap<String, String>, // os-arch -> download_url
    pub is_lts: bool,
    pub published_at: String,
}

/// GitHub Java 下载器
pub struct GitHubJavaDownloader {
    client: reqwest::Client,
    api_base_url: String,
}

impl GitHubJavaDownloader {
    /// 创建新的 GitHub Java 下载器
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            api_base_url: "https://api.github.com".to_string(),
        }
    }

    /// 获取可用的 Java 版本列表（从多个 Adoptium 仓库）
    pub async fn list_available_versions(&self) -> Result<Vec<GitHubJavaVersion>, String> {
        println!("🔍 正在从 GitHub 查询可用的 Java 版本...");

        // 尝试多个 Adoptium GitHub 仓库
        let repositories = vec![
            "adoptium/temurin21-binaries",
            "adoptium/temurin17-binaries",
            "adoptium/temurin11-binaries",
            "adoptium/temurin8-binaries",
        ];

        let mut all_versions = Vec::new();
        let mut seen_versions = std::collections::HashSet::new();

        for repo in repositories {
            println!("📦 检查仓库: {}", repo);

            let url = format!("{}/repos/{}/releases", self.api_base_url, repo);

            let response = self.client
                .get(&url)
                .header("User-Agent", "fnva/0.0.5")
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .await
                .map_err(|e| format!("请求 GitHub API 失败: {}", e))?;

            if !response.status().is_success() {
                println!("⚠️  仓库 {} 访问失败: {}", repo, response.status());
                continue;
            }

            let releases: Vec<GitHubJavaRelease> = match response.json().await {
                Ok(r) => r,
                Err(e) => {
                    println!("⚠️  解析仓库 {} 响应失败: {}", repo, e);
                    continue;
                }
            };

            for release in releases.into_iter().take(5) { // 每个仓库最多取5个版本
                // 跳过预发布版本
                if release.prerelease {
                    continue;
                }

                // 解析版本信息
                if let Ok(version_info) = self.parse_version_from_release(&release) {
                    // 避免重复版本
                    let version_key = format!("{}.{}.{}",
                        version_info.major,
                        version_info.minor.unwrap_or(0),
                        version_info.patch.unwrap_or(0));

                    if !seen_versions.contains(&version_key) {
                        seen_versions.insert(version_key);
                        all_versions.push(version_info);
                    }
                }
            }
        }

        // 按版本号排序
        all_versions.sort_by(|a, b| {
            b.major.cmp(&a.major)
                .then(b.minor.cmp(&a.minor))
                .then(b.patch.cmp(&a.patch))
        });

        println!("✅ 找到 {} 个可用版本", all_versions.len());
        Ok(all_versions)
    }

    /// 根据操作系统和架构获取下载链接
    pub async fn get_download_url(
        &self,
        version: &GitHubJavaVersion,
        os: &str,
        arch: &str
    ) -> Result<String, String> {
        let key = format!("{}-{}", os, arch);

        if let Some(url) = version.download_urls.get(&key) {
            return Ok(url.clone());
        }

        // 尝试匹配相似的配置
        for (platform_key, url) in &version.download_urls {
            if platform_key.starts_with(os) {
                println!("⚠️  使用相似的架构: {} -> {}", platform_key, key);
                return Ok(url.clone());
            }
        }

        Err(format!("未找到适合 {}-{} 的下载链接", os, arch))
    }

    /// 下载指定版本的 Java
    pub async fn download_java(
        &self,
        version: &GitHubJavaVersion,
        os: &str,
        arch: &str,
        progress_callback: impl Fn(u64, u64),
    ) -> Result<Vec<u8>, String> {
        let download_url = self.get_download_url(version, os, arch).await?;

        println!("📥 正在下载 Java {}...", version.version);
        println!("🔗 下载地址: {}", download_url);

        let response = self.client
            .get(&download_url)
            .header("User-Agent", "fnva/0.0.5")
            .send()
            .await
            .map_err(|e| format!("下载请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("下载失败: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut data = Vec::new();

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("下载流错误: {}", e))?;
            data.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            progress_callback(downloaded, total_size);
        }

        println!("✅ 下载完成，大小: {} MB", data.len() / (1024 * 1024));
        Ok(data)
    }

    /// 从 GitHub 发行版解析版本信息
    fn parse_version_from_release(&self, release: &GitHubJavaRelease) -> Result<GitHubJavaVersion, String> {
        let tag_name = &release.tag_name;

        // adoptium/jdk 的标签格式可能是：jdk-17.0.8+7, jdk-11.0.23+9 等
        let version_part = if let Some(version) = tag_name.strip_prefix("jdk-") {
            version
        } else {
            return Err(format!("无效的标签格式: {}", tag_name));
        };

        // 移除构建号部分，如 "17.0.8+7" -> "17.0.8"
        let clean_version = version_part.split('+').next().unwrap_or(version_part);

        let version_parts: Vec<&str> = clean_version.split('.').collect();
        if version_parts.len() < 2 {
            return Err("版本格式无效".to_string());
        }

        let major = version_parts[0].parse::<u32>()
            .map_err(|_| "无效的主版本号")?;
        let minor = version_parts.get(1).and_then(|s| s.parse::<u32>().ok());
        let patch = version_parts.get(2).and_then(|s| s.parse::<u32>().ok());

        // 判断是否为 LTS 版本
        let is_lts = [8, 11, 17, 21].contains(&major);

        // 解析下载链接
        let mut download_urls = HashMap::new();

        for asset in &release.assets {
            if let Some((os, arch)) = self.parse_os_arch_from_filename(&asset.name) {
                download_urls.insert(format!("{}-{}", os, arch), asset.browser_download_url.clone());
            }
        }

        Ok(GitHubJavaVersion {
            version: clean_version.to_string(),
            major,
            minor,
            patch,
            tag_name: tag_name.clone(),
            release_name: release.name.clone(),
            download_urls,
            is_lts,
            published_at: release.published_at.clone(),
        })
    }

    /// 从文件名解析操作系统和架构
    fn parse_os_arch_from_filename(&self, filename: &str) -> Option<(String, String)> {
        let filename_lower = filename.to_lowercase();

        // 解析操作系统
        let os = if filename_lower.contains("windows") || filename_lower.contains("win") {
            "windows"
        } else if filename_lower.contains("mac") || filename_lower.contains("darwin") {
            "macos"
        } else if filename_lower.contains("linux") {
            "linux"
        } else {
            return None;
        };

        // 解析架构
        let arch = if filename_lower.contains("x64") || filename_lower.contains("x86_64") {
            "x64"
        } else if filename_lower.contains("aarch64") || filename_lower.contains("arm64") {
            "aarch64"
        } else if filename_lower.contains("x86") || filename_lower.contains("i686") {
            "x86"
        } else {
            return None;
        };

        Some((os.to_string(), arch.to_string()))
    }

    /// 获取当前系统信息
    pub fn get_current_system_info() -> (String, String) {
        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "unknown"
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else if cfg!(target_arch = "x86") {
            "x86"
        } else {
            "unknown"
        };

        (os.to_string(), arch.to_string())
    }

    /// 根据版本规格查找版本
    pub async fn find_version_by_spec(
        &self,
        spec: &str
    ) -> Result<GitHubJavaVersion, String> {
        let versions = self.list_available_versions().await?;

        let spec_cleaned = spec.trim().to_lowercase()
            .replace("jdk", "")  // 移除 jdk 前缀
            .replace("java", ""); // 移除 java 前缀

        if spec_cleaned == "lts" || spec_cleaned == "latest-lts" {
            // 返回最新的 LTS 版本
            for version in versions {
                if version.is_lts {
                    return Ok(version);
                }
            }
            return Err("未找到 LTS 版本".to_string());
        } else if spec_cleaned == "latest" || spec_cleaned == "newest" {
            // 返回最新版本
            return versions.into_iter().next()
                .ok_or("未找到可用版本".to_string());
        }

        // 尝试解析为主版本号 (支持 jdk8, java8, 8 等格式)
        if let Ok(major) = spec_cleaned.parse::<u32>() {
            for version in versions {
                if version.major == major {
                    return Ok(version);
                }
            }
            return Err(format!("未找到 Java {}", major));
        }

        // 尝试精确匹配
        for version in versions {
            if version.version == spec_cleaned || version.tag_name == spec_cleaned {
                return Ok(version);
            }
        }

        Err(format!("未找到版本: {}", spec))
    }
}

impl Default for GitHubJavaDownloader {
    fn default() -> Self {
        Self::new()
    }
}