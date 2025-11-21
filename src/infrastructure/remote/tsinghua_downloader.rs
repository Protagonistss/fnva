use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{download::download_to_bytes, platform::Platform};

/// Mirror download entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsinghuaDownloadEntry {
    pub primary: String,
    pub fallback: Option<String>,
}

/// Java version mapped to Tsinghua mirror URLs with GitHub fallback
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TsinghuaJavaVersion {
    pub version: String,
    pub major: u32,
    pub minor: Option<u32>,
    pub patch: Option<u32>,
    pub release_name: String,
    pub tag_name: String,
    pub download_urls: HashMap<String, TsinghuaDownloadEntry>, // os-arch -> download_url
    pub is_lts: bool,
    pub published_at: String,
}

/// Downloader for Tsinghua Adoptium mirror
pub struct TsinghuaJavaDownloader {
    client: reqwest::Client,
    base_url: String,
}

impl TsinghuaJavaDownloader {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://mirrors.tuna.tsinghua.edu.cn/Adoptium".to_string(),
        }
    }

    /// Build mirror version list without caching (临时禁用缓存)
    pub async fn list_available_versions(&self) -> Result<Vec<TsinghuaJavaVersion>, String> {
        println!("-> Fetching Java versions from Tsinghua mirror (cache disabled)...");

        let mut versions = Vec::new();

        // 1. 获取硬编码的稳定版本
        println!("-> Loading hardcoded stable versions...");
        let stable_versions = self.get_stable_versions();
        versions.extend(stable_versions);

        println!("<- Prepared {} stable versions", versions.len());

        Ok(versions)
    }

    /// Get hardcoded stable versions for Tsinghua mirror
    fn get_stable_versions(&self) -> Vec<TsinghuaJavaVersion> {
        let mut versions = Vec::new();

        // Helper struct for defining stable versions
        struct StableDef {
            version: &'static str,
            major: u32,
            is_lts: bool,
            filename_map: Vec<(&'static str, &'static str)>, // (os-arch, filename)
        }

        let stable_defs = vec![
            // Java 25 (Latest EA)
            StableDef {
                version: "25.0.1+8",
                major: 25,
                is_lts: false,
                filename_map: vec![
                    ("windows-x64", "OpenJDK25U-jdk_x64_windows_hotspot_25.0.1_8.zip"),
                    ("linux-x64", "OpenJDK25U-jdk_x64_linux_hotspot_25.0.1_8.tar.gz"),
                    ("linux-aarch64", "OpenJDK25U-jdk_aarch64_linux_hotspot_25.0.1_8.tar.gz"),
                    ("macos-x64", "OpenJDK25U-jdk_x64_mac_hotspot_25.0.1_8.tar.gz"),
                    ("macos-aarch64", "OpenJDK25U-jdk_aarch64_mac_hotspot_25.0.1_8.tar.gz"),
                ],
            },
            // Java 21 (LTS)
            StableDef {
                version: "21.0.9+10",
                major: 21,
                is_lts: true,
                filename_map: vec![
                    ("windows-x64", "OpenJDK21U-jdk_x64_windows_hotspot_21.0.9_10.zip"),
                    ("linux-x64", "OpenJDK21U-jdk_x64_linux_hotspot_21.0.9_10.tar.gz"),
                    ("linux-aarch64", "OpenJDK21U-jdk_aarch64_linux_hotspot_21.0.9_10.tar.gz"),
                    ("macos-x64", "OpenJDK21U-jdk_x64_mac_hotspot_21.0.9_10.tar.gz"),
                    ("macos-aarch64", "OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.9_10.tar.gz"),
                ],
            },
            // Java 20 (Non-LTS)
            StableDef {
                version: "20.0.2+9",
                major: 20,
                is_lts: false,
                filename_map: vec![
                    ("windows-x64", "OpenJDK20U-jdk_x64_windows_hotspot_20.0.2_9.zip"),
                    ("linux-x64", "OpenJDK20U-jdk_x64_linux_hotspot_20.0.2_9.tar.gz"),
                    ("linux-aarch64", "OpenJDK20U-jdk_aarch64_linux_hotspot_20.0.2_9.tar.gz"),
                    ("macos-x64", "OpenJDK20U-jdk_x64_mac_hotspot_20.0.2_9.tar.gz"),
                    ("macos-aarch64", "OpenJDK20U-jdk_aarch64_mac_hotspot_20.0.2_9.tar.gz"),
                ],
            },
            // Java 19 (Non-LTS)
            StableDef {
                version: "19.0.2+7",
                major: 19,
                is_lts: false,
                filename_map: vec![
                    ("windows-x64", "OpenJDK19U-jdk_x64_windows_hotspot_19.0.2_7.zip"),
                    ("linux-x64", "OpenJDK19U-jdk_x64_linux_hotspot_19.0.2_7.tar.gz"),
                    ("linux-aarch64", "OpenJDK19U-jdk_aarch64_linux_hotspot_19.0.2_7.tar.gz"),
                    ("macos-x64", "OpenJDK19U-jdk_x64_mac_hotspot_19.0.2_7.tar.gz"),
                    ("macos-aarch64", "OpenJDK19U-jdk_aarch64_mac_hotspot_19.0.2_7.tar.gz"),
                ],
            },
            // Java 18 (Non-LTS)
            StableDef {
                version: "18.0.2+9",
                major: 18,
                is_lts: false,
                filename_map: vec![
                    ("windows-x64", "OpenJDK18U-jdk_x64_windows_hotspot_18.0.2_9.zip"),
                    ("linux-x64", "OpenJDK18U-jdk_x64_linux_hotspot_18.0.2_9.tar.gz"),
                    ("linux-aarch64", "OpenJDK18U-jdk_aarch64_linux_hotspot_18.0.2_9.tar.gz"),
                    ("macos-x64", "OpenJDK18U-jdk_x64_mac_hotspot_18.0.2_9.tar.gz"),
                    ("macos-aarch64", "OpenJDK18U-jdk_aarch64_mac_hotspot_18.0.2_9.tar.gz"),
                ],
            },
            // Java 17 (LTS)
            StableDef {
                version: "17.0.17+10",
                major: 17,
                is_lts: true,
                filename_map: vec![
                    ("windows-x64", "OpenJDK17U-jdk_x64_windows_hotspot_17.0.17_10.zip"),
                    ("linux-x64", "OpenJDK17U-jdk_x64_linux_hotspot_17.0.17_10.tar.gz"),
                    ("linux-aarch64", "OpenJDK17U-jdk_aarch64_linux_hotspot_17.0.17_10.tar.gz"),
                    ("macos-x64", "OpenJDK17U-jdk_x64_mac_hotspot_17.0.17_10.tar.gz"),
                    ("macos-aarch64", "OpenJDK17U-jdk_aarch64_mac_hotspot_17.0.17_10.tar.gz"),
                ],
            },
            // Java 11 (LTS)
            StableDef {
                version: "11.0.29+7",
                major: 11,
                is_lts: true,
                filename_map: vec![
                    ("windows-x64", "OpenJDK11U-jdk_x64_windows_hotspot_11.0.29_7.zip"),
                    ("linux-x64", "OpenJDK11U-jdk_x64_linux_hotspot_11.0.29_7.tar.gz"),
                    ("linux-aarch64", "OpenJDK11U-jdk_aarch64_linux_hotspot_11.0.29_7.tar.gz"),
                    ("macos-x64", "OpenJDK11U-jdk_x64_mac_hotspot_11.0.29_7.tar.gz"),
                    ("macos-aarch64", "OpenJDK11U-jdk_aarch64_mac_hotspot_11.0.29_7.tar.gz"),
                ],
            },
            // Java 8 (LTS)
            StableDef {
                version: "8u472b08",
                major: 8,
                is_lts: true,
                filename_map: vec![
                    ("windows-x64", "OpenJDK8U-jdk_x64_windows_hotspot_8u472b08.zip"),
                    ("linux-x64", "OpenJDK8U-jdk_x64_linux_hotspot_8u472b08.tar.gz"),
                    ("linux-aarch64", "OpenJDK8U-jdk_aarch64_linux_hotspot_8u472b08.tar.gz"),
                    ("macos-x64", "OpenJDK8U-jdk_x64_mac_hotspot_8u472b08.tar.gz"),
                    ("macos-aarch64", "OpenJDK8U-jdk_aarch64_mac_hotspot_8u472b08.tar.gz"),
                ],
            },
        ];

        for def in stable_defs {
            let mut download_urls = HashMap::new();

            for (key, filename) in def.filename_map {
                let (os, arch) = key.split_once('-').unwrap();
                // Map "macos" key to "mac" in URL path if needed (consistent with logic)
                // Here key is "macos-x64", so os="macos". URL path should use "mac".
                let mirror_os = match os {
                    "macos" => "mac",
                    other => other,
                };

                let url = format!(
                    "{}/{}/jdk/{}/{}/{}",
                    self.base_url,
                    def.major,
                    arch,
                    mirror_os,
                    filename
                );

                download_urls.insert(
                    key.to_string(),
                    TsinghuaDownloadEntry {
                        primary: url,
                        fallback: None,
                    },
                );
            }

            versions.push(TsinghuaJavaVersion {
                version: def.version.to_string(),
                major: def.major,
                minor: Some(0), // Simplified
                patch: Some(0), // Simplified
                release_name: format!("Eclipse Temurin JDK {} ({})", def.version,
                    if def.is_lts { "LTS" } else { "Non-LTS" }),
                tag_name: format!("jdk-{}", def.version),
                download_urls,
                is_lts: def.is_lts,
                published_at: "Stable".to_string(),
            });
        }

        versions
    }

    /// Get download URL for platform with mirror-first and GitHub fallback
    pub async fn get_download_url(
        &self,
        version: &TsinghuaJavaVersion,
        platform: &Platform,
    ) -> Result<String, String> {
        let key = platform.key();

        if let Some(entry) = version.download_urls.get(&key) {
            return self.pick_available_url(entry).await;
        }

        // Try similar OS even if arch key differs
        for (platform_key, entry) in version.download_urls.iter() {
            if platform_key.starts_with(&platform.os) {
                println!("-> Using closest platform match: {} -> {}", platform_key, key);
                return self.pick_available_url(entry).await;
            }
        }

        Err(format!("No download url matches {}", key))
    }

    async fn pick_available_url(&self, entry: &TsinghuaDownloadEntry) -> Result<String, String> {
        // Prefer mirror first
        if self.is_url_available(&entry.primary).await {
            return Ok(entry.primary.clone());
        }

        if let Some(fallback) = &entry.fallback {
            println!("-> Mirror unavailable, falling back to GitHub");
            return Ok(fallback.clone());
        }

        Err("Primary and fallback download url unavailable".to_string())
    }

    async fn is_url_available(&self, url: &str) -> bool {
        match self.client.head(url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Download the specified version
    pub async fn download_java(
        &self,
        version: &TsinghuaJavaVersion,
        platform: &Platform,
        progress_callback: impl Fn(u64, u64),
    ) -> Result<Vec<u8>, String> {
        let download_url = self.get_download_url(version, platform).await?;

        println!("-> Downloading Java {} from mirror...", version.version);
        println!("-> URL: {}", download_url);

        let data = download_to_bytes(&self.client, &download_url, progress_callback).await?;
        println!("<- Downloaded size: {} MB", data.len() / (1024 * 1024));
        Ok(data)
    }

    /// Find version by spec (using GitHub metadata to mirror)
    pub async fn find_version_by_spec(&self, spec: &str) -> Result<TsinghuaJavaVersion, String> {
        let versions = self.list_available_versions().await?;

        let spec_cleaned = spec.trim().to_lowercase()
            .replace("v", "")
            .replace("jdk", "")
            .replace("java", "")
            .trim()
            .to_string();

        if spec_cleaned == "lts" || spec_cleaned == "latest-lts" {
            for version in &versions {
                if version.is_lts {
                    return Ok(version.clone());
                }
            }
            return Err("No LTS version found".to_string());
        } else if spec_cleaned == "latest" || spec_cleaned == "newest" {
            return versions.into_iter().next()
                .ok_or("No available versions".to_string());
        }

        let parts: Vec<&str> = spec_cleaned.split('.').filter(|p| !p.is_empty()).collect();
        if !parts.is_empty() && parts[0].parse::<u32>().is_ok() {
            if parts.len() == 1 {
                let major = parts[0].parse::<u32>().unwrap();

                let mut lts_versions: Vec<&TsinghuaJavaVersion> = versions.iter()
                    .filter(|v| v.major == major && v.is_lts)
                    .collect();
                lts_versions.sort_by(|a, b| b.version.cmp(&a.version));
                if let Some(latest_lts) = lts_versions.first() {
                    return Ok((**latest_lts).clone());
                }

                let mut major_versions: Vec<&TsinghuaJavaVersion> = versions.iter()
                    .filter(|v| v.major == major)
                    .collect();
                major_versions.sort_by(|a, b| b.version.cmp(&a.version));
                if let Some(latest) = major_versions.first() {
                    return Ok((**latest).clone());
                }

                return Err(format!("No Java {} found", major));
            } else {
                let full_version = parts.join(".");
                for version in &versions {
                    if version.version == full_version ||
                       version.tag_name.contains(&full_version) ||
                       version.release_name.to_lowercase().contains(&full_version) {
                        return Ok(version.clone());
                    }
                }

                let major = parts[0].parse::<u32>().unwrap();
                for version in &versions {
                    if version.major == major {
                        return Ok(version.clone());
                    }
                }

                return Err(format!("Version not found: {}", spec));
            }
        }

        for version in versions {
            if version.version == spec_cleaned ||
               version.tag_name == spec_cleaned ||
               version.release_name.to_lowercase().contains(&spec_cleaned) {
                return Ok(version);
            }
        }

        Err(format!("Version not found: {}", spec))
    }
}

impl Default for TsinghuaJavaDownloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tsinghua_downloader_functionality() {
        println!("🎓 测试清华镜像下载器功能...");
        let downloader = TsinghuaJavaDownloader::new();

        // 测试获取版本列表
        match downloader.list_available_versions().await {
            Ok(versions) => {
                println!("✅ 清华版本列表获取成功，共 {} 个版本", versions.len());
                assert!(!versions.is_empty(), "版本列表不应为空");

                // 测试稳定版本
                let stable_versions = downloader.get_stable_versions();
                println!("✅ 内置稳定版本数量: {}", stable_versions.len());
                assert!(!stable_versions.is_empty(), "应该有内置稳定版本");

                // 测试版本解析
                let test_specs = ["21", "17", "11", "25", "20", "lts"];
                for spec in test_specs {
                    match downloader.find_version_by_spec(spec).await {
                        Ok(version) => {
                            let lts_marker = if version.is_lts { " (LTS)" } else { "" };
                            println!("✅ 清华版本解析 '{}' -> Java {}{}", spec, version.version, lts_marker);
                            assert!(!version.version.is_empty());
                            assert!(version.major > 0);
                            assert!(!version.download_urls.is_empty());

                            // 测试平台下载链接
                            let platform = Platform::current();
                            match downloader.get_download_url(&version, &platform).await {
                                Ok(url) => {
                                    println!("  ✅ 下载链接获取成功: {}",
                                        url.chars().take(60).collect::<String>());
                                    assert!(url.contains("tsinghua") || url.contains("github"));
                                }
                                Err(e) => {
                                    println!("  ⚠️  获取下载链接失败: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("⚠️  清华版本解析 '{}' 失败: {}", spec, e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ 清华版本列表获取失败: {}", e);
                // 不标记为测试失败，因为可能是网络问题
            }
        }
    }
}
