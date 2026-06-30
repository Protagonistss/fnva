use crate::config::{Config, EnvironmentSource, MavenEnvironment};
use crate::infrastructure::installer::generic;
use crate::infrastructure::remote::platform::Platform;
use crate::infrastructure::tool_protocol::{AssetModel, ToolDescriptor, ToolDownloader};
use std::fs;

use super::downloader::MavenDownloader;
use super::validator::{locate_maven_home, validate_maven_home};

/// Maven 安装管理器
pub struct MavenInstaller;

/// Maven 工具描述符(供通用 installer 骨架参数化)
pub const MAVEN_DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    id: "maven",
    display_name: "Maven",
    asset_model: AssetModel::SingleArchive,
    install_subdir: "packages/maven",
    home_validator: validate_maven_home,
    locate_home: locate_maven_home,
};

impl MavenInstaller {
    /// 安装指定版本的 Maven
    pub async fn install_maven(
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        println!("Installing Maven {version_spec}...");

        // 检查本地是否已存在解压好的包
        if let Some(home) = Self::check_local(version_spec, config)? {
            println!("Found local Maven package: {version_spec}");
            println!("Using local install: {home}");
            return Self::complete_installation(
                version_spec,
                config,
                auto_switch,
                &home,
                version_spec,
            )
            .await;
        }

        let mirrors = config.mirrors.maven.clone();
        let mirror_names: Vec<&str> = mirrors
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.name.as_str())
            .collect();
        println!("Mirrors: {}", mirror_names.join(" -> "));

        let downloader = MavenDownloader::new(mirrors);
        let resolved = downloader
            .find_version_by_spec(version_spec)
            .await
            .map_err(|e| format!("{e:?}"))?;
        println!(
            "Resolved version: {} ({})",
            resolved.version, resolved.display
        );

        let platform = Platform::current();
        let maven_home = generic::download_and_install(
            &downloader,
            &resolved,
            &platform,
            version_spec,
            &MAVEN_DESCRIPTOR,
        )
        .await?;

        Self::complete_installation(
            version_spec,
            config,
            auto_switch,
            &maven_home,
            &resolved.version,
        )
        .await
    }

    /// 完成安装:写配置 + 可选自动切换(设 current_maven_env)。
    /// 注:真正的 shell 环境变量注入由 `fnva maven use` 完成(shell 集成)。
    async fn complete_installation(
        install_name: &str,
        config: &mut Config,
        auto_switch: bool,
        maven_home: &str,
        version: &str,
    ) -> Result<String, String> {
        if let Some(existing) = config.get_maven_env(install_name) {
            println!("Maven {version} is already installed");
            println!("Path: {}", existing.maven_home);
            return Ok(existing.maven_home.clone());
        }

        let description = format!("Apache Maven {version} ({maven_home})");
        config.add_maven_env(MavenEnvironment {
            name: install_name.to_string(),
            maven_home: maven_home.to_string(),
            description,
            source: EnvironmentSource::Manual,
        })?;
        config.save()?;

        println!("Maven {version} installed");
        println!("Path: {maven_home}");

        if auto_switch {
            let _ = config.set_current_maven_env(install_name.to_string());
            config.save()?;
            println!("Set current Maven to {version}");
            println!("Run 'fnva maven use {install_name}' in a new terminal to activate");
        }

        Ok(maven_home.to_string())
    }

    /// 检查本地 `~/.fnva/packages/maven/{spec}` 是否已存在解压好的 Maven。
    fn check_local(version_spec: &str, config: &Config) -> Result<Option<String>, String> {
        if config.get_maven_env(version_spec).is_some() {
            return Ok(None); // 已在配置中,走正常流程(会提示已安装)
        }
        let fnva_dir = crate::infrastructure::paths::tool_packages_dir("maven")?;
        let maven_home = fnva_dir.join(version_spec);
        if maven_home.exists() {
            let actual = locate_maven_home(&maven_home)?;
            return Ok(Some(actual));
        }
        Ok(None)
    }

    /// 卸载 Maven 版本(仅限 fnva 管理的安装)
    pub fn uninstall_maven(version_name: &str, config: &mut Config) -> Result<(), String> {
        let maven_env = config
            .get_maven_env(version_name)
            .ok_or_else(|| format!("Maven environment '{version_name}' not found"))?
            .clone();

        if !maven_env.maven_home.contains(".fnva/packages/maven") {
            return Err("Only fnva-managed Maven installations can be uninstalled".to_string());
        }

        println!("Uninstalling Maven {version_name}...");
        println!("Removing: {}", maven_env.maven_home);
        fs::remove_dir_all(&maven_env.maven_home)
            .map_err(|e| format!("Failed to remove install dir: {e}"))?;
        config.remove_maven_env(version_name)?;

        if config
            .default_maven_env
            .as_ref()
            .is_some_and(|d| d == version_name)
        {
            config.default_maven_env = None;
        }
        if config
            .current_maven_env
            .as_ref()
            .is_some_and(|c| c == version_name)
        {
            config.current_maven_env = None;
        }
        config.save()?;
        println!("Maven {version_name} uninstalled");
        Ok(())
    }
}
