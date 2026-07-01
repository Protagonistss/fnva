use super::downloader::JavaDownloader;
use crate::config::Config;
use crate::infrastructure::installer::generic;
use crate::infrastructure::remote::platform::Platform;
use crate::infrastructure::tool_protocol::{
    AssetModel, ResolvedVersion, ToolDescriptor, ToolDownloader,
};
use std::fs;
use std::path::Path;

/// Java 安装管理器
pub struct JavaInstaller;

/// Java 工具描述符(供通用 installer 骨架参数化)
pub const JAVA_DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    id: "java",
    display_name: "Java",
    asset_model: AssetModel::PerPlatform,
    install_subdir: "packages/java",
    home_validator: crate::utils::validate_java_home,
    locate_home: JavaInstaller::find_installed_java,
};

impl JavaInstaller {
    /// 安装指定版本的 Java（使用模板化下载器）
    pub async fn install_java(
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        crate::cli::print::action(&format!("Installing java {version_spec}"));

        if let Ok(java_home) = Self::check_local_java_package(version_spec, config) {
            crate::cli::print::step("Source", "local package");
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

        let mirrors = config.mirrors.java.clone();
        let mirror_names: Vec<&str> = mirrors
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.name.as_str())
            .collect();
        crate::cli::print::step("Mirrors", &mirror_names.join(" -> "));

        let downloader = JavaDownloader::new(mirrors);
        let res =
            Self::install_with_downloader(&downloader, version_spec, config, auto_switch).await;

        match res {
            Ok(java_home) => Ok(java_home),
            Err(e) => Err(format!("All mirrors failed: {e}")),
        }
    }

    async fn install_with_downloader(
        downloader: &dyn ToolDownloader,
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        let resolved = match downloader.find_version_by_spec(version_spec).await {
            Ok(version) => {
                crate::cli::print::step(
                    "Resolved",
                    &format!("{} ({})", version.version, version.display),
                );
                version
            }
            Err(_) => {
                crate::cli::print::step("Resolved", "using latest...");
                downloader
                    .list_available_versions()
                    .await
                    .map_err(|e| format!("{e:?}"))?
                    .into_iter()
                    .next()
                    .ok_or_else(|| "No versions available".to_string())?
            }
        };

        let platform = Platform::current();
        let java_home =
            Self::download_and_install(downloader, &resolved, &platform, version_spec).await?;
        Self::complete_installation_simple(
            version_spec,
            config,
            auto_switch,
            &java_home,
            &resolved.version,
            &resolved.display,
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
        let install_name = version_spec.to_string();

        // Already installed - return success with info message
        if let Some(existing) = config.get_java_env(&install_name) {
            crate::cli::print::step("Status", "Already installed");
            return Ok(existing.java_home.clone());
        }

        let description = format!("Java {version} ({java_home})");
        config.add_java_env(crate::config::JavaEnvironment {
            name: install_name.clone(),
            java_home: java_home.to_string(),
            description,
            source: crate::config::EnvironmentSource::Manual,
        })?;
        config.save()?;

        if auto_switch {
            crate::cli::print::step("Auto-switch", &format!("to {version}..."));
            if let Err(e) = Self::switch_to_java(&install_name, config) {
                crate::cli::print::warn(&format!("Auto-switch failed: {e}"));
            } else {
                crate::cli::print::step("Status", &format!("Switched to java {version}"));
            }
        }

        Ok(java_home.to_string())
    }

    async fn download_and_install(
        downloader: &dyn ToolDownloader,
        version: &ResolvedVersion,
        platform: &Platform,
        env_name: &str,
    ) -> Result<String, String> {
        generic::download_and_install(downloader, version, platform, env_name, &JAVA_DESCRIPTOR)
            .await
    }

    /// 查找已安装的 Java 目录
    fn find_installed_java(install_dir: &Path) -> Result<String, String> {
        // 检查是否直接包含 Java 安装
        if crate::utils::validate_java_home(&install_dir.to_string_lossy()) {
            return Ok(install_dir.to_string_lossy().to_string());
        }

        // macOS tar.gz --strip-components=1 后结构为 Contents/Home/bin/java
        // 直接检查这个路径
        let contents_home = install_dir.join("Contents").join("Home");
        if contents_home.exists()
            && crate::utils::validate_java_home(&contents_home.to_string_lossy())
        {
            return Ok(contents_home.to_string_lossy().to_string());
        }

        // 搜索子目录（Windows .zip 和 Linux tar.gz 均可能有子目录）
        for entry in
            fs::read_dir(install_dir).map_err(|e| format!("Failed to read install dir: {e}"))?
        {
            let entry = entry.map_err(|e| format!("Failed to read dir entry: {e}"))?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            // 直接子目录就是 JAVA_HOME（Windows .zip: jdk-17.0.9+9/bin/java.exe）
            if crate::utils::validate_java_home(&path.to_string_lossy()) {
                return Ok(path.to_string_lossy().to_string());
            }

            // macOS .zip/.tar.gz 子目录下可能有 Contents/Home
            let sub_contents_home = path.join("Contents").join("Home");
            if sub_contents_home.exists()
                && crate::utils::validate_java_home(&sub_contents_home.to_string_lossy())
            {
                return Ok(sub_contents_home.to_string_lossy().to_string());
            }
        }

        Err("No valid Java installation found".to_string())
    }

    /// 切换到指定的 Java 版本
    fn switch_to_java(version_name: &str, config: &Config) -> Result<(), String> {
        let java_env = config
            .get_java_env(version_name)
            .ok_or_else(|| format!("Java environment '{version_name}' not found"))?;

        // 验证 Java Home 路径
        if !crate::utils::validate_java_home(&java_env.java_home) {
            return Err(format!("Invalid JAVA_HOME: {}", java_env.java_home));
        }

        println!(
            "Switching to Java: {} ({})",
            version_name, java_env.java_home
        );
        println!("Run 'fnva java use {version_name}' in a new terminal to activate");

        Ok(())
    }

    /// 列出可安装的 Java 版本
    pub async fn list_installable_versions() -> Result<Vec<String>, String> {
        let config = crate::infrastructure::config::Config::load()
            .map_err(|e| format!("Failed to load config: {e}"))?;

        let mirrors = config.mirrors.java.clone();
        let downloader = JavaDownloader::new(mirrors);

        let versions = ToolDownloader::list_available_versions(&downloader)
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
                .entry(version.major.unwrap_or(0))
                .or_default()
                .push(version_str);
        }

        let mut major_versions: Vec<_> = versions_by_major.keys().cloned().collect();
        major_versions.sort_by(|a, b| b.cmp(a));

        result.push("Available versions (* = LTS):".to_string());
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
                    "        ... and {} more",
                    versions_for_major.len() - 8
                ));
            }
        }

        let total_versions: usize = versions.len();
        let lts_count: usize = versions.iter().filter(|v| v.is_lts).count();
        result.push("".to_string());
        result.push(format!("Total: {total_versions} versions, {lts_count} LTS"));

        Ok(result)
    }

    /// 卸载 Java 版本
    pub fn uninstall_java(version_name: &str, config: &mut Config) -> Result<(), String> {
        let java_env = config
            .get_java_env(version_name)
            .ok_or_else(|| format!("Java environment '{version_name}' not found"))?;

        let java_home = &java_env.java_home;

        // 检查是否是 fnva 管理的安装
        if !java_home.contains(".fnva/packages/java") {
            return Err("Only fnva-managed Java installations can be uninstalled".to_string());
        }

        crate::cli::print::action(&format!("Uninstalling java {version_name}"));
        crate::cli::print::step("Removing", java_home);

        // 删除安装目录
        fs::remove_dir_all(java_home).map_err(|e| format!("Failed to remove install dir: {e}"))?;

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

        crate::cli::print::success(&format!("java {version_name} uninstalled"));
        Ok(())
    }

    /// 检查本地是否已有对应的Java包
    fn check_local_java_package(version_spec: &str, config: &Config) -> Result<String, String> {
        let fnva_dir = crate::infrastructure::paths::tool_packages_dir("java")?;

        if !fnva_dir.exists() {
            return Err("Local Java packages directory not found. Install Java first".to_string());
        }

        // 如果在配置中已经存在该环境，则不认为是可用的本地包
        if config.get_java_env(version_spec).is_some() {
            return Err(format!("Java {version_spec} already exists in config"));
        }

        let java_home = fnva_dir.join(version_spec);

        // 如果本地包目录存在，则查找实际的Java安装目录
        if java_home.exists() {
            // 查找实际的Java安装目录（可能在其子目录中）
            let actual_java_home = Self::find_installed_java(&java_home)?;
            return Ok(actual_java_home);
        }

        Err(format!("Local Java package not found: {version_spec}"))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_version_spec() {
        assert!(matches!(
            crate::environments::java::parse_version_spec("21").unwrap(),
            crate::environments::java::VersionSpec::Major(21)
        ));
        assert!(matches!(
            crate::environments::java::parse_version_spec("lts").unwrap(),
            crate::environments::java::VersionSpec::LatestLts
        ));
        assert!(matches!(
            crate::environments::java::parse_version_spec("8-11").unwrap(),
            crate::environments::java::VersionSpec::Range(8, 11)
        ));
        assert!(matches!(
            crate::environments::java::parse_version_spec("v21").unwrap(),
            crate::environments::java::VersionSpec::Major(21)
        ));
    }
}
