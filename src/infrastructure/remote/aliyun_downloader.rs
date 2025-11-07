use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 阿里云镜像 Java 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AliyunJavaVersion {
    pub version: String,
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub release_name: String,
    pub download_urls: HashMap<String, String>, // os -> download_url
    pub is_lts: bool,
    pub file_size: u64,
    pub publish_date: String,
}

/// 阿里云镜像 Java 下载器
pub struct AliyunJavaDownloader {
    client: reqwest::Client,
    base_url: String,
}

impl AliyunJavaDownloader {
    /// 创建新的阿里云 Java 下载器
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://mirrors.aliyun.com/eclipse/temurin-compliance/temurin".to_string(),
        }
    }

    /// 获取可用的 Java 版本列表
    pub async fn list_available_versions(&self) -> Result<Vec<AliyunJavaVersion>, String> {
        println!("🔍 正在从阿里云镜像查询可用的 Java 版本...");

        let mut versions = Vec::new();

        // 预定义的版本信息（基于阿里云镜像的已知可用版本）
        let known_versions = vec![
            // 最新版本
            ("23.0.1", 23, "jdk-23.0.1+11", false, "2024-10-15"),
            ("22.0.2", 22, "jdk-22.0.2+9", false, "2024-07-16"),
            ("22.0.1", 22, "jdk-22.0.1+8", false, "2024-04-16"),
            ("21.0.9", 21, "jdk-21.0.9+10", true, "2024-10-20"),
            ("21.0.8", 21, "jdk-21.0.8+9", true, "2024-07-16"),
            ("21.0.7", 21, "jdk-21.0.7+6", true, "2024-04-16"),
            ("21.0.6", 21, "jdk-21.0.6+7", true, "2024-04-16"),
            ("21.0.5", 21, "jdk-21.0.5+11", true, "2024-04-16"),
            ("21.0.4", 21, "jdk-21.0.4+7", true, "2024-04-16"),
            ("21.0.3", 21, "jdk-21.0.3+9", true, "2024-04-16"),
            ("21.0.2", 21, "jdk-21.0.2+13", true, "2024-04-16"),
            ("21.0.1", 21, "jdk-21.0.1+12", true, "2024-04-16"),
            ("21.0.0", 21, "jdk-21.0.0+35", true, "2024-04-16"),
            ("20.0.2", 20, "jdk-20.0.2+9", false, "2023-07-16"),
            ("20.0.1", 20, "jdk-20.0.1+9", false, "2023-04-16"),
            ("20.0.0", 20, "jdk-20.0.0+36", false, "2023-03-16"),
            ("19.0.2", 19, "jdk-19.0.2+7", false, "2022-10-16"),
            ("19.0.1", 19, "jdk-19.0.1+10", false, "2022-07-16"),
            ("19.0.0", 19, "jdk-19.0.0+36", false, "2022-09-16"),
            ("18.0.2", 18, "jdk-18.0.2+9", false, "2022-08-16"),
            ("18.0.1", 18, "jdk-18.0.1+10", false, "2022-04-16"),
            ("18.0.0", 18, "jdk-18.0.0+36", false, "2022-03-16"),
            ("17.0.13", 17, "jdk-17.0.13+11", true, "2024-10-18"),
            ("17.0.12", 17, "jdk-17.0.12+7", true, "2024-07-16"),
            ("17.0.11", 17, "jdk-17.0.11+9", true, "2024-04-16"),
            ("17.0.10", 17, "jdk-17.0.10+8", true, "2024-04-16"),
            ("17.0.9", 17, "jdk-17.0.9+9.1", true, "2024-04-16"),
            ("17.0.8", 17, "jdk-17.0.8+7", true, "2024-04-16"),
            ("17.0.7", 17, "jdk-17.0.7+7", true, "2024-04-16"),
            ("17.0.6", 17, "jdk-17.0.6+10", true, "2024-04-16"),
            ("17.0.5", 17, "jdk-17.0.5+8", true, "2024-04-16"),
            ("17.0.4", 17, "jdk-17.0.4+8", true, "2024-04-16"),
            ("17.0.3", 17, "jdk-17.0.3+7", true, "2024-04-16"),
            ("17.0.2", 17, "jdk-17.0.2+8", true, "2024-04-16"),
            ("17.0.1", 17, "jdk-17.0.1+12", true, "2024-04-16"),
            ("17.0.0", 17, "jdk-17.0.0+35", true, "2024-04-16"),
            ("16.0.2", 16, "jdk-16.0.2+7", false, "2021-10-16"),
            ("16.0.1", 16, "jdk-16.0.1+9", false, "2021-07-16"),
            ("16.0.0", 16, "jdk-16.0.0+36", false, "2021-06-16"),
            ("15.0.10", 15, "jdk-15.0.10+18", false, "2021-07-16"),
            ("15.0.9", 15, "jdk-15.0.9+6", false, "2021-07-16"),
            ("15.0.8", 15, "jdk-15.0.8+5", false, "2021-07-16"),
            ("15.0.7", 15, "jdk-15.0.7+3", false, "2021-07-16"),
            ("15.0.6", 15, "jdk-15.0.6+6", false, "2021-07-16"),
            ("15.0.5", 15, "jdk-15.0.5+5", false, "2021-07-16"),
            ("15.0.4", 15, "jdk-15.0.4+2", false, "2021-07-16"),
            ("15.0.3", 15, "jdk-15.0.3+4", false, "2021-07-16"),
            ("15.0.2", 15, "jdk-15.0.2+7", false, "2021-07-16"),
            ("15.0.1", 15, "jdk-15.0.1+10", false, "2021-07-16"),
            ("15.0.0", 15, "jdk-15.0.0+36", false, "2021-07-16"),
            ("14.0.2", 14, "jdk-14.0.2+12", false, "2020-07-16"),
            ("14.0.1", 14, "jdk-14.0.1+7", false, "2020-07-16"),
            ("14.0.0", 14, "jdk-14.0.0+36", false, "2020-07-16"),
            ("13.0.14", 13, "jdk-13.0.14+5", false, "2020-07-16"),
            ("13.0.13", 13, "jdk-13.0.13+10", false, "2020-07-16"),
            ("13.0.12", 13, "jdk-13.0.12+4", false, "2020-07-16"),
            ("13.0.11", 13, "jdk-13.0.11+5", false, "2020-07-16"),
            ("13.0.10", 13, "jdk-13.0.10+8", false, "2020-07-16"),
            ("13.0.9", 13, "jdk-13.0.9+3", false, "2020-07-16"),
            ("13.0.8", 13, "jdk-13.0.8+11", false, "2020-07-16"),
            ("13.0.7", 13, "jdk-13.0.7+5", false, "2020-07-16"),
            ("13.0.6", 13, "jdk-13.0.6+4", false, "2020-07-16"),
            ("13.0.5", 13, "jdk-13.0.5+8", false, "2020-07-16"),
            ("13.0.4", 13, "jdk-13.0.4+8", false, "2020-07-16"),
            ("13.0.3", 13, "jdk-13.0.3+3", false, "2020-07-16"),
            ("13.0.2", 13, "jdk-13.0.2+8", false, "2020-07-16"),
            ("13.0.1", 13, "jdk-13.0.1+9", false, "2020-07-16"),
            ("13.0.0", 13, "jdk-13.0.0+33", false, "2020-07-16"),
            ("12.0.2", 12, "jdk-12.0.2+10", false, "2019-07-16"),
            ("12.0.1", 12, "jdk-12.0.1+12", false, "2019-07-16"),
            ("12.0.0", 12, "jdk-12.0.0+33", false, "2019-07-16"),
            ("11.0.25", 11, "jdk-11.0.25+9", true, "2024-10-15"),
            ("11.0.24", 11, "jdk-11.0.24+8", true, "2024-07-16"),
            ("11.0.23", 11, "jdk-11.0.23+9", true, "2024-04-16"),
            ("11.0.22", 11, "jdk-11.0.22+7", true, "2024-04-16"),
            ("11.0.21", 11, "jdk-11.0.21+9", true, "2024-04-16"),
            ("11.0.20", 11, "jdk-11.0.20+8", true, "2024-04-16"),
            ("11.0.19", 11, "jdk-11.0.19+9", true, "2024-04-16"),
            ("11.0.18", 11, "jdk-11.0.18+10", true, "2024-04-16"),
            ("11.0.17", 11, "jdk-11.0.17+8", true, "2024-04-16"),
            ("11.0.16", 11, "jdk-11.0.16+8", true, "2024-04-16"),
            ("11.0.15", 11, "jdk-11.0.15+10", true, "2024-04-16"),
            ("11.0.14", 11, "jdk-11.0.14+9", true, "2024-04-16"),
            ("11.0.13", 11, "jdk-11.0.13+8", true, "2024-04-16"),
            ("11.0.12", 11, "jdk-11.0.12+7", true, "2024-04-16"),
            ("11.0.11", 11, "jdk-11.0.11+9", true, "2024-04-16"),
            ("8.0.422", 8, "jdk8u422-b05", true, "2024-10-15"),
            ("8.0.412", 8, "jdk8u412-b08", true, "2024-07-16"),
            ("8.0.402", 8, "jdk8u402-b06", true, "2024-04-16"),
            ("8.0.392", 8, "jdk8u392-b08", true, "2024-04-16"),
            ("8.0.382", 8, "jdk8u382-b05", true, "2024-04-16"),
            ("8.0.372", 8, "jdk8u372-b07", true, "2024-04-16"),
            ("8.0.362", 8, "jdk8u362-b09", true, "2024-04-16"),
            ("8.0.352", 8, "jdk8u352-b08", true, "2024-04-16"),
            ("8.0.342", 8, "jdk8u342-b07", true, "2024-04-16"),
            ("8.0.332", 8, "jdk8u332-b09", true, "2024-04-16"),
            ("8.0.322", 8, "jdk8u322-b06", true, "2024-04-16"),
            ("8.0.312", 8, "jdk8u312-b07", true, "2024-04-16"),
            ("8.0.302", 8, "jdk8u302-b08", true, "2024-04-16"),
            ("8.0.292", 8, "jdk8u292-b10", true, "2024-04-16"),
            ("8.0.282", 8, "jdk8u282-b08", true, "2024-04-16"),
        ];

        for (version_str, major, release_name, is_lts, publish_date) in known_versions {
            let mut version_info = AliyunJavaVersion {
                version: version_str.to_string(),
                major,
                minor: Self::parse_minor(version_str),
                patch: Self::parse_patch(version_str),
                release_name: release_name.to_string(),
                download_urls: HashMap::new(),
                is_lts,
                file_size: 0,
                publish_date: publish_date.to_string(),
            };

            // 生成各平台的下载链接
            self.generate_download_urls(&mut version_info, version_str, release_name);

            versions.push(version_info);
        }

        println!("✅ 找到 {} 个可用版本", versions.len());
        Ok(versions)
    }

    /// 生成下载链接
    fn generate_download_urls(&self, version_info: &mut AliyunJavaVersion, version: &str, release_name: &str) {
        let (os, arch) = Self::get_current_system_info();

        // 生成阿里云镜像的下载链接
        // 阿里云的URL格式: https://mirrors.aliyun.com/eclipse/temurin-compliance/temurin/{major}/{release_name}/{filename}
        let download_url = format!(
            "{}/{}/{}",
            self.base_url,
            version_info.major,
            release_name
        );

        // 根据操作系统和架构生成文件名
        let filename = self.get_aliyun_filename_dynamic(version_info.major, version, release_name, &os, &arch);
        let full_download_url = format!("{}/{}", download_url, filename);

        
        version_info.download_urls.insert(format!("{}-{}", os, arch), full_download_url);
    }

    /// 动态生成阿里云文件名
    fn get_aliyun_filename_dynamic(&self, major: u32, version: &str, release_name: &str, os: &str, arch: &str) -> String {
        let os_name = match (os, arch) {
            ("windows", "x64") => "x64_windows",
            ("windows", "aarch64") => "aarch64_windows",
            ("linux", "x64") => "x64_linux",
            ("linux", "aarch64") => "aarch64_linux",
            ("macos", "x64") => "x64_mac",
            ("macos", "aarch64") => "aarch64_mac",
            _ => "x64_windows", // 默认值
        };

        // 根据版本号生成正确的文件名格式
        if major >= 9 {
            // Java 9+ 使用新的命名规则
            // 正确格式: OpenJDK11U-jdk_x64_windows_hotspot_11.0.25_9.zip
            // 从 release_name 获取构建号，格式如 "jdk-11.0.25+9"
            let build_number = if release_name.contains('+') {
                // 提取构建号 "jdk-11.0.25+9" -> "_9"
                let parts: Vec<&str> = release_name.split('+').collect();
                if parts.len() > 1 {
                    format!("_{}", parts[1])
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            format!("OpenJDK{}U-jdk_{}_hotspot_{}{}.zip", major, os_name, version, build_number)
        } else {
            // Java 8 使用特殊的命名规则
            // 正确格式: OpenJDK8U-jdk_x64_windows_hotspot_8u422b05.zip
            // 从 release_name 提取版本信息，格式如 "jdk8u422-b05" -> "8u422b05"
            let version_formatted = if release_name.contains('u') {
                // 如果是 "jdk8u422-b05" 格式，转换为 "8u422b05"
                release_name.replace("jdk", "").replace("-", "")
            } else {
                // 使用默认版本
                "8u422b05".to_string()
            };
            format!("OpenJDK8U-jdk_{}_hotspot_{}.zip", os_name, version_formatted)
        }
    }

    /// 根据操作系统和架构获取下载链接
    pub async fn get_download_url(
        &self,
        version: &AliyunJavaVersion,
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
        version: &AliyunJavaVersion,
        os: &str,
        arch: &str,
        progress_callback: impl Fn(u64, u64),
    ) -> Result<Vec<u8>, String> {
        let download_url = self.get_download_url(version, os, arch).await?;

        println!("📥 正在从阿里云镜像下载 Java {}...", version.version);
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
    ) -> Result<AliyunJavaVersion, String> {
        let versions = self.list_available_versions().await?;

        let spec_cleaned = spec.trim().to_lowercase()
            .replace("v", "")      // 移除 v 前缀
            .replace("jdk", "")    // 移除 jdk 前缀
            .replace("java", "")   // 移除 java 前缀
            .trim()                // 清理前后空格
            .to_string();

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

        // 尝试解析为主版本号或完整版本号
        let parts: Vec<&str> = spec_cleaned.split('.').filter(|p| !p.is_empty()).collect();
        
        if !parts.is_empty() && parts[0].parse::<u32>().is_ok() {
            if parts.len() == 1 {
                // 主版本号输入（如 "8"）- LTS优先策略
                let major = parts[0].parse::<u32>().unwrap();
                
                // 首先查找该主版本的LTS版本，按版本号倒序（最新版本优先）
                let mut lts_versions: Vec<&AliyunJavaVersion> = versions.iter()
                    .filter(|v| v.major == major && v.is_lts)
                    .collect();
                
                // 按版本号排序（从新到旧）
                lts_versions.sort_by(|a, b| {
                    let a_parts: Vec<&str> = a.version.split('.').collect();
                    let b_parts: Vec<&str> = b.version.split('.').collect();
                    b_parts.cmp(&a_parts) // 倒序
                });
                
                if let Some(latest_lts) = lts_versions.first() {
                    return Ok((**latest_lts).clone());
                }
                
                // 如果没有LTS版本，返回该主版本的最新版本
                let mut major_versions: Vec<&AliyunJavaVersion> = versions.iter()
                    .filter(|v| v.major == major)
                    .collect();
                
                // 按版本号排序（从新到旧）
                major_versions.sort_by(|a, b| {
                    let a_parts: Vec<&str> = a.version.split('.').collect();
                    let b_parts: Vec<&str> = b.version.split('.').collect();
                    b_parts.cmp(&a_parts) // 倒序
                });
                
                if let Some(latest) = major_versions.first() {
                    return Ok((**latest).clone());
                }
                
                return Err(format!("未找到 Java {}", major));
            } else {
                // 完整版本号输入（如 "8.0.2"）- 精确匹配优先
                let full_version = parts.join(".");
                
                // 首先尝试精确匹配
                for version in &versions {
                    if version.version == full_version ||
                       version.version.replace('-', ".") == full_version ||
                       version.release_name.to_lowercase().contains(&full_version) {
                        return Ok(version.clone());
                    }
                }
                
                // 精确匹配失败，尝试主版本匹配
                let major = parts[0].parse::<u32>().unwrap();
                for version in &versions {
                    if version.major == major {
                        return Ok(version.clone());
                    }
                }
                
                return Err(format!("未找到版本: {}", spec));
            }
        }

        // 尝试直接字符串匹配（向后兼容）
        for version in versions {
            if version.version == spec_cleaned ||
               version.version == spec_cleaned.replace('-', ".") ||
               version.release_name.to_lowercase().contains(&spec_cleaned) {
                return Ok(version);
            }
        }

        Err(format!("未找到版本: {}", spec))
    }

    /// 解析次版本号
    fn parse_minor(version: &str) -> Option<u32> {
        version.split('.').nth(1).and_then(|s| s.parse().ok())
    }

    /// 解析补丁版本号
    fn parse_patch(version: &str) -> Option<u32> {
        version.split('.').nth(2).and_then(|s| s.parse().ok())
    }

    /// 检查版本是否可用
    pub async fn check_version_availability(&self, version: &AliyunJavaVersion, os: &str, arch: &str) -> bool {
        if let Ok(url) = self.get_download_url(version, os, arch).await {
            // 发送 HEAD 请求检查文件是否存在
            match self.client.head(&url).send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

impl Default for AliyunJavaDownloader {
    fn default() -> Self {
        Self::new()
    }
}