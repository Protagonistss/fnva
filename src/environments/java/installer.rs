use crate::config::Config;
use crate::infrastructure::remote::{JavaDownloader, Platform, UnifiedJavaVersion};
use crate::infrastructure::remote::template_downloader::TemplateDownloader;
use std::fs;
use std::path::Path;

/// Java 安装管理器
pub struct JavaInstaller;

impl JavaInstaller {
    /// 安装指定版本的 Java（使用模板化下载器）
    pub async fn install_java(
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        println!("🚀 正在准备安装 Java {version_spec}...");

        // 检查本地是否已有对应的 Java 包
        if let Ok(java_home) = Self::check_local_java_package(version_spec, config) {
            println!("🎉 检测到本地Java包: {version_spec}");
            println!("📁 使用本地安装: {java_home}");
            return Self::complete_installation_simple(
                version_spec, config, auto_switch, &java_home, "local", "local",
            ).await;
        }

        let mirrors = config.mirrors.java.clone();
        let mirror_names: Vec<&str> = mirrors.iter().filter(|m| m.enabled).map(|m| m.name.as_str()).collect();
        println!("📋 下载源优先级: {}", mirror_names.join(" -> "));

        let downloader = TemplateDownloader::new(mirrors);
        let res = Self::install_with_downloader(
            &downloader, version_spec, config, auto_switch,
        ).await;

        match res {
            Ok(java_home) => Ok(java_home),
            Err(e) => Err(format!("所有镜像源均失败: {e}")),
        }
    }

    /// 使用模板化下载器安装 Java
    async fn install_with_downloader(
        downloader: &TemplateDownloader,
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        let java_version = match downloader.find_version_by_spec(version_spec).await {
            Ok(version) => {
                println!("解析到版本: {} ({})", version.version, version.release_name);
                version
            }
            Err(_) => {
                println!("无法从 '{version_spec}' 解析版本，使用最新版本");
                downloader.list_available_versions().await
                    .map_err(|e| format!("{e:?}"))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| "无法获取最新版本".to_string())?
            }
        };

        let platform = Platform::current();
        let java_home = Self::download_and_install(downloader, &java_version, &platform, version_spec).await?;
        Self::complete_installation_simple(
            version_spec, config, auto_switch, &java_home, &java_version.version, &java_version.release_name,
        ).await
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
            return Err(format!("Java {version} 已经安装"));
        }

        // 添加到配置
        let description = format!("Java {version} ({java_home})");
        config.add_java_env(crate::config::JavaEnvironment {
            name: install_name.clone(),
            java_home: java_home.to_string(),
            description,
            source: crate::config::EnvironmentSource::Manual,
        })?;
        config.save()?;

        println!("✅ Java {version} 安装成功！");
        println!("📁 安装路径: {java_home}");

        // 自动切换
        if auto_switch {
            println!("🔄 自动切换到 Java {version}");
            if let Err(e) = Self::switch_to_java(&install_name, config) {
                println!("⚠️  自动切换失败: {e}");
            } else {
                println!("✅ 已切换到 Java {version}");
            }
        }

        Ok(java_home.to_string())
    }

    async fn download_and_install(
        downloader: &dyn JavaDownloader,
        version_info: &UnifiedJavaVersion,
        platform: &Platform,
        env_name: &str,
    ) -> Result<String, String> {
        let pb =
            crate::infrastructure::installer::utils::create_progress_bar().unwrap_or_else(|_| {
                // If progress bar creation fails, create a simple one
                let pb = indicatif::ProgressBar::new_spinner();
                pb.set_style(
                    indicatif::ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap()
                        .progress_chars("=>-"),
                );
                pb
            });

        // 克隆进度条以便在回调中使用
        let pb_clone = pb.clone();

        let target = downloader
            .download_java(
                version_info,
                platform,
                Box::new(move |downloaded, total| {
                    if total > 0 {
                        // 设置总长度并更新进度
                        if pb_clone.length().unwrap_or(0) == 0 {
                            pb_clone.set_length(total);
                        }
                        pb_clone.set_position(downloaded);
                    } else {
                        // 如果未知总大小，显示下载的字节数
                        pb_clone.set_message(format!("已下载: {} MB", downloaded / (1024 * 1024)));
                        pb_clone.tick();
                    }
                }),
            )
            .await
            .map_err(|e| format!("下载失败: {e:?}"))?;
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

        fs::create_dir_all(&fnva_dir).map_err(|e| format!("创建安装目录失败: {e}"))?;

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
        for entry in fs::read_dir(install_dir).map_err(|e| format!("读取安装目录失败: {e}"))?
        {
            let entry = entry.map_err(|e| format!("读取目录项失败: {e}"))?;
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
            .ok_or_else(|| format!("Java 环境 '{version_name}' 不存在"))?;

        // 验证 Java Home 路径
        if !crate::utils::validate_java_home(&java_env.java_home) {
            return Err(format!("无效的 JAVA_HOME 路径: {}", java_env.java_home));
        }

        println!("🔄 切换到 Java: {} ({})", version_name, java_env.java_home);
        println!("💡 请在新的终端中运行以下命令来激活环境:");
        println!("   fnva java use {version_name}");

        Ok(())
    }

    /// 列出可安装的 Java 版本
    pub async fn list_installable_versions() -> Result<Vec<String>, String> {
        let config = crate::infrastructure::config::Config::load()
            .map_err(|e| format!("加载配置失败: {e}"))?;

        let mirrors = config.mirrors.java.clone();
        let downloader = TemplateDownloader::new(mirrors);

        let versions = downloader
            .list_available_versions()
            .await
            .map_err(|e| format!("{e:?}"))?;

        let mut result = Vec::new();

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
                .or_default()
                .push(version_str);
        }

        let mut major_versions: Vec<_> = versions_by_major.keys().cloned().collect();
        major_versions.sort_by(|a, b| b.cmp(a));

        result.push(format!(
            "🌟 所有可用版本 (带*的为LTS版本):"
        ));
        result.push("".to_string());

        for major in major_versions.iter().take(15) {
            let versions_for_major = &versions_by_major[major];
            let mut line = format!("Java {major}: ");

            for (i, version) in versions_for_major.iter().take(8).enumerate() {
                if i > 0 && i % 4 == 0 {
                    result.push(line.clone());
                    line = "        ".to_string();
                }
                line.push_str(&format!("{version:<12}"));
            }
            result.push(line);

            if versions_for_major.len() > 8 {
                result.push(format!(
                    "        ... 还有 {} 个版本",
                    versions_for_major.len() - 8
                ));
            }
        }

        let total_versions: usize = versions.len();
        let lts_count: usize = versions.iter().filter(|v| v.is_lts).count();
        result.push("".to_string());
        result.push(format!(
            "📊 总计: {total_versions} 个版本，其中 {lts_count} 个LTS版本"
        ));

        Ok(result)
    }

    /// 卸载 Java 版本
    pub fn uninstall_java(version_name: &str, config: &mut Config) -> Result<(), String> {
        let java_env = config
            .get_java_env(version_name)
            .ok_or_else(|| format!("Java 环境 '{version_name}' 不存在"))?;

        let java_home = &java_env.java_home;

        // 检查是否是 fnva 管理的安装
        if !java_home.contains(".fnva/java-packages") {
            return Err("只能卸载通过 fnva 安装的 Java 版本".to_string());
        }

        println!("🗑️  正在卸载 Java {version_name}...");
        println!("📁 删除路径: {java_home}");

        // 删除安装目录
        fs::remove_dir_all(java_home).map_err(|e| format!("删除安装目录失败: {e}"))?;

        // 从配置中移除
        config.remove_java_env(version_name)?;

        // 如果删除的是默认环境，清理默认环境设置
        if config
            .default_java_env
            .as_ref()
            .is_some_and(|default| default == version_name)
        {
            config.default_java_env = None;
        }

        config.save()?;

        println!("✅ Java {version_name} 卸载成功");
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
            return Err(format!("Java {version_spec} 已经在配置中存在"));
        }

        let java_home = fnva_dir.join(version_spec);

        // 如果本地包目录存在，则查找实际的Java安装目录
        if java_home.exists() {
            // 查找实际的Java安装目录（可能在其子目录中）
            let actual_java_home = Self::find_installed_java(&java_home)?;
            return Ok(actual_java_home);
        }

        Err(format!("本地未找到Java包: {version_spec}"))
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
