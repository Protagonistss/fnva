use crate::config::Config;
use crate::infrastructure::remote::{JavaDownloader, Platform, UnifiedJavaVersion};
use std::fs;
use std::path::Path;

/// Java 安装管理器
pub struct JavaInstaller;

impl JavaInstaller {
    /// 安装指定版本的 Java（使用配置的下载器）
    pub async fn install_java(
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        println!("🚀 正在准备安装 Java {}...", version_spec);

        // 在开始安装前，检查本地是否已有对应的Java包（避免重复下载）
        if let Ok(java_home) = Self::check_local_java_package(version_spec, config) {
            println!("🎉 检测到本地Java包: {}", version_spec);
            println!("📁 使用本地安装: {}", java_home);

            // 直接完成安装流程（使用本地包）
            return Self::complete_installation_simple(
                version_spec,
                config,
                auto_switch,
                &java_home,
                "local",
                "local",
            )
            .await;
        }

        let primary = config.repositories.java.downloader.clone();
        let mut chain = Vec::new();
        chain.push(primary);
        chain.extend(config.repositories.java.fallback.clone());

        println!("📋 下载源优先级链: {}", chain.join(" -> "));

        let mut last_err: Option<String> = None;
        for source in chain {
            let downloader: Box<dyn JavaDownloader> = match source.as_str() {
                "github" => Box::new(crate::remote::GitHubJavaDownloader::new()),
                "aliyun" => Box::new(crate::remote::AliyunJavaDownloader::new()),
                "tsinghua" => Box::new(crate::remote::TsinghuaJavaDownloader::new()),
                _ => {
                    println!("⚠️  未知的下载器类型: '{}' , 跳过", source);
                    continue;
                }
            };

            let res = Self::install_with_downloader(
                downloader,
                version_spec,
                config,
                auto_switch,
                &source,
            )
            .await;

            match res {
                Ok(java_home) => return Ok(java_home),
                Err(e) => {
                    println!("↩️  源 '{}' 失败: {}", source, e);
                    last_err = Some(e);
                    continue;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| "所有下载源均失败".to_string()))
    }

    /// 使用通用下载器安装 Java
    async fn install_with_downloader(
        downloader: Box<dyn JavaDownloader>,
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
        source_name: &str,
    ) -> Result<String, String> {
        // 尝试从自定义名称中解析版本，如果失败则使用最新版本
        let java_version = match downloader.find_version_by_spec(version_spec).await {
            Ok(version) => {
                println!("解析到版本: {} ({})", version.version, version.release_name);
                version
            }
            Err(_) => {
                println!("无法从 '{}' 解析版本，使用最新版本", version_spec);
                // 获取最新版本
                downloader
                    .list_available_versions()
                    .await
                    .map_err(|e| format!("{:?}", e))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| "无法获取最新版本".to_string())?
            }
        };

        println!("使用 {} 下载器: {}", source_name, java_version.release_name);

        let platform = Platform::current();
        // 恢复使用用户输入的原始格式
        let java_home =
            Self::download_and_install(&downloader, &java_version, &platform, version_spec).await?;
        Self::complete_installation_simple(
            version_spec,
            config,
            auto_switch,
            &java_home,
            &java_version.version,
            &java_version.release_name,
        )
        .await
    }

    /// 完成安装流程（简单下载器）
    async fn complete_installation_simple(
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
        java_home: &str,
        version: &str,
        _release_name: &str,
    ) -> Result<String, String> {
        // 使用用户输入的原始名称，确保名称唯一性
        let install_name = version_spec.to_string();

        // 检查是否已安装
        if config.get_java_env(&install_name).is_some() {
            return Err(format!("Java {} 已经安装", version));
        }

        // 添加到配置
        let description = format!("Java {} ({})", version, java_home);
        config.add_java_env(crate::config::JavaEnvironment {
            name: install_name.clone(),
            java_home: java_home.to_string(),
            description,
            source: crate::config::EnvironmentSource::Manual,
        })?;
        config.save()?;

        println!("✅ Java {} 安装成功！", version);
        println!("📁 安装路径: {}", java_home);

        // 自动切换
        if auto_switch {
            println!("🔄 自动切换到 Java {}", version);
            if let Err(e) = Self::switch_to_java(&install_name, config) {
                println!("⚠️  自动切换失败: {}", e);
            } else {
                println!("✅ 已切换到 Java {}", version);
            }
        }

        Ok(java_home.to_string())
    }

    async fn download_and_install(
        downloader: &Box<dyn JavaDownloader>,
        version_info: &UnifiedJavaVersion,
        platform: &Platform,
        env_name: &str,
    ) -> Result<String, String> {
        let pb = crate::infrastructure::installer::utils::create_progress_bar().unwrap_or_else(|_| {
            // If progress bar creation fails, create a simple one
            indicatif::ProgressBar::new_spinner()
        });

        // Wrap callback in Arc/Mutex or ensure Send+Sync?
        // The trait requires Send+Sync for callback.
        // indicatif ProgressBar is Send+Sync (usually, via Arc internally).

        let target = downloader
            .download_java(
                version_info,
                platform,
                Box::new(move |_downloaded, _total| {
                    // Progress callback - temporarily simplified
                }),
            )
            .await
            .map_err(|e| format!("下载失败: {:?}", e))?;
        pb.finish_with_message("下载完成");

        // 下载器现在直接下载到文件，避免内存占用
        let file_path = match target {
            crate::remote::DownloadTarget::File(p) => {
                // 文件已经下载完成，直接使用
                std::path::PathBuf::from(p)
            }
            crate::remote::DownloadTarget::Bytes(_) => {
                // 保留对旧实现的兼容性（虽然现在不会用到）
                return Err("不支持内存下载模式，请使用文件下载".to_string());
            }
        };

        let java_home = Self::install_archive(&file_path, &version_info.version, env_name).await?;

        if !crate::utils::validate_java_home(&java_home) {
            return Err("安装验证失败".to_string());
        }

        Ok(java_home)
    }

    /// 安装压缩包（跨平台）
    async fn install_archive(
        archive_path: &Path,
        _version: &str,
        env_name: &str,
    ) -> Result<String, String> {
        // 获取 fnva 安装目录
        let fnva_dir = dirs::home_dir()
            .ok_or("无法获取用户主目录")?
            .join(".fnva")
            .join("java-packages");

        fs::create_dir_all(&fnva_dir).map_err(|e| format!("创建安装目录失败: {}", e))?;

        let java_home = fnva_dir.join(env_name);

        // 解压文件
        if archive_path.to_str().unwrap().ends_with(".zip") {
            crate::infrastructure::installer::utils::extract_zip(archive_path, &java_home)?;
        } else {
            crate::infrastructure::installer::utils::extract_tar_gz(archive_path, &java_home)?;
        }

        // 查找实际的 JAVA_HOME（可能在子目录中）
        let actual_home = Self::find_installed_java(&java_home)?;
        Ok(actual_home)
    }

    /// 查找已安装的 Java 目录
    fn find_installed_java(install_dir: &Path) -> Result<String, String> {
        // 检查是否直接包含 Java 安装
        if crate::utils::validate_java_home(&install_dir.to_string_lossy()) {
            return Ok(install_dir.to_string_lossy().to_string());
        }

        // 搜索子目录
        for entry in fs::read_dir(install_dir).map_err(|e| format!("读取安装目录失败: {}", e))?
        {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();

            if path.is_dir() && crate::utils::validate_java_home(&path.to_string_lossy()) {
                return Ok(path.to_string_lossy().to_string());
            }

            // 对于 macOS，检查 Contents/Home
            if cfg!(target_os = "macos") {
                let contents_home = path.join("Contents").join("Home");
                if contents_home.exists()
                    && crate::utils::validate_java_home(&contents_home.to_string_lossy())
                {
                    return Ok(contents_home.to_string_lossy().to_string());
                }
            }
        }

        Err("未找到有效的 Java 安装目录".to_string())
    }

    /// 切换到指定的 Java 版本
    fn switch_to_java(version_name: &str, config: &Config) -> Result<(), String> {
        let java_env = config
            .get_java_env(version_name)
            .ok_or_else(|| format!("Java 环境 '{}' 不存在", version_name))?;

        // 验证 Java Home 路径
        if !crate::utils::validate_java_home(&java_env.java_home) {
            return Err(format!("无效的 JAVA_HOME 路径: {}", java_env.java_home));
        }

        println!("🔄 切换到 Java: {} ({})", version_name, java_env.java_home);
        println!("💡 请在新的终端中运行以下命令来激活环境:");
        println!("   fnva java use {}", version_name);

        Ok(())
    }

    /// 列出可安装的 Java 版本
    pub async fn list_installable_versions() -> Result<Vec<String>, String> {
        let config = crate::infrastructure::config::Config::load()
            .map_err(|e| format!("加载配置失败: {}", e))?;

        let downloader_type = &config.repositories.java.downloader;

        let downloader: Box<dyn JavaDownloader> = match downloader_type.as_str() {
            "github" => Box::new(crate::remote::GitHubJavaDownloader::new()),
            "tsinghua" => Box::new(crate::remote::TsinghuaJavaDownloader::new()),
            "aliyun" => Box::new(crate::remote::AliyunJavaDownloader::new()),
            _ => Box::new(crate::remote::AliyunJavaDownloader::new()), // Default fallback
        };

        let versions = downloader
            .list_available_versions()
            .await
            .map_err(|e| format!("{:?}", e))?;

        let mut result = Vec::new();

        // Format output similar to before but using UnifiedJavaVersion
        use std::collections::HashMap;
        let mut versions_by_major: HashMap<u32, Vec<String>> = HashMap::new();

        for version in &versions {
            let version_str = if version.is_lts {
                format!("{}*", version.version)
            } else {
                version.version.to_string()
            };
            versions_by_major
                .entry(version.major)
                .or_insert_with(Vec::new)
                .push(version_str);
        }

        let mut major_versions: Vec<_> = versions_by_major.keys().cloned().collect();
        major_versions.sort_by(|a, b| b.cmp(a));

        result.push(format!(
            "🌟 所有可用版本 (源: {}, 带*的为LTS版本):",
            downloader_type
        ));
        result.push("".to_string());

        for major in major_versions.iter().take(15) {
            let versions_for_major = &versions_by_major[major];
            let mut line = format!("Java {}: ", major);

            for (i, version) in versions_for_major.iter().take(8).enumerate() {
                if i > 0 && i % 4 == 0 {
                    result.push(line.clone());
                    line = format!("        ");
                }
                line.push_str(&format!("{:<12}", version));
            }
            result.push(line);

            if versions_for_major.len() > 8 {
                result.push(format!(
                    "        ... 还有 {} 个版本",
                    versions_for_major.len() - 8
                ));
            }
        }

        let total_versions: usize = versions.iter().count();
        let lts_count: usize = versions.iter().filter(|v| v.is_lts).count();
        result.push("".to_string());
        result.push(format!(
            "📊 总计: {} 个版本，其中 {} 个LTS版本",
            total_versions, lts_count
        ));

        Ok(result)
    }

    /// 卸载 Java 版本
    pub fn uninstall_java(version_name: &str, config: &mut Config) -> Result<(), String> {
        let java_env = config
            .get_java_env(version_name)
            .ok_or_else(|| format!("Java 环境 '{}' 不存在", version_name))?;

        let java_home = &java_env.java_home;

        // 检查是否是 fnva 管理的安装
        if !java_home.contains(".fnva/java-packages") {
            return Err("只能卸载通过 fnva 安装的 Java 版本".to_string());
        }

        println!("🗑️  正在卸载 Java {}...", version_name);
        println!("📁 删除路径: {}", java_home);

        // 删除安装目录
        fs::remove_dir_all(java_home).map_err(|e| format!("删除安装目录失败: {}", e))?;

        // 从配置中移除
        config.remove_java_env(version_name)?;

        // 如果删除的是默认环境，清理默认环境设置
        if config
            .default_java_env
            .as_ref()
            .map_or(false, |default| default == version_name)
        {
            config.default_java_env = None;
        }

        config.save()?;

        println!("✅ Java {} 卸载成功", version_name);
        Ok(())
    }

    /// 检查本地是否已有对应的Java包
    fn check_local_java_package(version_spec: &str, config: &Config) -> Result<String, String> {
        let fnva_dir = dirs::home_dir()
            .ok_or("无法获取用户主目录")?
            .join(".fnva")
            .join("java-packages");

        if !fnva_dir.exists() {
            return Err("本地Java包目录不存在，请先安装Java".to_string());
        }

        // 如果在配置中已经存在该环境，则不认为是可用的本地包
        if config.get_java_env(version_spec).is_some() {
            return Err(format!("Java {} 已经在配置中存在", version_spec));
        }

        let java_home = fnva_dir.join(version_spec);

        // 如果本地包目录存在，则查找实际的Java安装目录
        if java_home.exists() {
            // 查找实际的Java安装目录（可能在其子目录中）
            let actual_java_home = Self::find_installed_java(&java_home)?;
            return Ok(actual_java_home);
        }

        Err(format!("本地未找到Java包: {}", version_spec))
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_version_manager_parsing() {
        let _version_manager =
            crate::environments::java::VersionManager::new("https://api.adoptium.net/v3");

        // 测试版本解析
        assert!(matches!(
            crate::environments::java::VersionManager::parse_version_spec("21").unwrap(),
            crate::environments::java::VersionSpec::Major(21)
        ));
        assert!(matches!(
            crate::environments::java::VersionManager::parse_version_spec("lts").unwrap(),
            crate::environments::java::VersionSpec::LatestLts
        ));
        assert!(matches!(
            crate::environments::java::VersionManager::parse_version_spec("8-11").unwrap(),
            crate::environments::java::VersionSpec::Range(8, 11)
        ));
    }

    #[test]
    fn test_legacy_parse_version_spec() {
        // 这些测试现在通过异步版本管理器处理
        // 保留一些基本的格式测试
        let version_spec =
            crate::environments::java::VersionManager::parse_version_spec("v21").unwrap();
        assert!(matches!(
            version_spec,
            crate::environments::java::VersionSpec::Major(21)
        ));
    }
}
