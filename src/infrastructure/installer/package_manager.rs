use crate::config::Config;
use crate::remote::{JavaVersionInfo, RemoteManager};
use crate::utils::validate_java_home;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Java 资源包管理器
pub struct JavaPackageManager;

impl JavaPackageManager {
    /// 安装 Java 资源包（下载并解压）
    pub async fn install_java_package(
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        println!("🚀 正在准备安装 Java 资源包 {}...", version_spec);

        // 解析版本规格并规范化环境名称
        let java_version = Self::parse_version_spec(version_spec)?;
        let env_name = Self::normalize_env_name(version_spec);

        // 获取版本信息
        let version_info = Self::get_version_info(&java_version).await?;

        // 检查是否已安装
        if config.get_java_env(&env_name).is_some() {
            return Err(format!("Java {} 环境已经安装", env_name));
        }

        // 获取合适的下载链接
        let download_url = Self::get_package_download_url(&version_info)?;
        println!("📦 选择资源包格式: {}", Self::get_package_type(&download_url));

        // 下载和解压
        let package_path = Self::download_and_extract_package(&download_url, &version_info).await?;

        // 验证安装
        if !validate_java_home(&package_path) {
            return Err("资源包验证失败".to_string());
        }

        // 添加到配置
        let description = format!("Java {} Package (Portable)", version_info.version);
        config.add_java_env(crate::config::JavaEnvironment {
            name: env_name.clone(),
            java_home: package_path.clone(),
            description,
            source: crate::config::EnvironmentSource::Manual,
        })?;
        config.save()?;

        println!("✅ Java {} 资源包安装成功！", version_info.version);
        println!("📁 安装路径: {}", package_path);

        // 自动切换
        if auto_switch {
            println!("🔄 自动切换到 Java {}", env_name);
            if let Err(e) = Self::switch_to_java(&env_name, config) {
                println!("⚠️  自动切换失败: {}", e);
            } else {
                println!("✅ 已切换到 Java {}", env_name);
            }
        }

        Ok(package_path)
    }

    /// 解析版本规格
    fn parse_version_spec(version_spec: &str) -> Result<u32, String> {
        let cleaned = version_spec
            .trim()
            .to_lowercase()
            .replace("v", "")
            .replace("java", "")
            .replace("jdk", "")
            .replace("pkg", "")
            .replace("package", "");

        if let Ok(version) = cleaned.parse::<u32>() {
            match version {
                8 | 11 | 17 | 21 => Ok(version),
                _ => Err(format!(
                    "不支持的 Java 版本: {}. 支持的版本: 8, 11, 17, 21",
                    version
                )),
            }
        } else {
            Err(format!("无效的版本规格: {}", version_spec))
        }
    }

    /// 规范化环境名称（直接使用用户输入的名称）
    fn normalize_env_name(version_spec: &str) -> String {
        version_spec.trim().to_string()
    }

    /// 获取版本信息
    async fn get_version_info(major_version: &u32) -> Result<JavaVersionInfo, String> {
        // 加载配置以获取仓库列表
        let config = Config::load().map_err(|e| format!("加载配置失败: {}", e))?;

        // 使用配置中的 Java 仓库
        let repositories = &config.repositories.java.repositories;

        for repo in repositories {
            println!("🔍 尝试从 {} 获取版本信息...", repo);

            let mut remote_manager = RemoteManager::new();
            match remote_manager.list_java_versions(
                Some(repo),
                Some(*major_version),
                None,
                None,
            ).await {
                Ok(mut versions) => {
                    if let Some(version) = versions.pop() {
                        println!("✅ 成功获取版本信息: {}", version.version);
                        return Ok(version);
                    } else {
                        println!("⚠️  {} 中未找到 Java {} 版本", repo, major_version);
                    }
                }
                Err(e) => {
                    println!("⚠️  从 {} 获取版本信息失败: {}", repo, e);
                }
            }
        }

        Err(format!("所有源都无法获取 Java {} 的版本信息", major_version))
    }

    /// 获取资源包下载链接（使用从远程源获取的链接）
    fn get_package_download_url(version_info: &JavaVersionInfo) -> Result<String, String> {
        // 直接使用从远程源获取的 download_url
        if let Some(download_url) = &version_info.download_url {
            println!("🔗 使用下载链接: {}", download_url);
            Ok(download_url.clone())
        } else {
            Err("未找到可用的下载链接".to_string())
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
            "macos" => "macos",
            "linux" => "linux",
            _ => "linux",
        };

        let version = if cfg!(target_os = "windows") {
            format!("{}-{}", os, arch)
        } else {
            format!("{}-{}", os, arch)
        };

        (version, arch.to_string(), os.to_string())
    }

    /// 获取包类型
    fn get_package_type(url: &str) -> &'static str {
        if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            "TAR.GZ (Portable)"
        } else if url.ends_with(".zip") {
            "ZIP (Portable)"
        } else if url.ends_with(".msi") {
            "MSI (Installer)"
        } else {
            "Unknown"
        }
    }

    /// 下载并解压资源包
    async fn download_and_extract_package(
        download_url: &str,
        version_info: &JavaVersionInfo,
    ) -> Result<String, String> {
        // 创建临时目录
        let temp_dir = TempDir::new()
            .map_err(|e| format!("创建临时目录失败: {}", e))?;

        let file_name = Self::extract_filename_from_url(download_url);
        let file_path = temp_dir.path().join(&file_name);

        // 下载文件
        Self::download_file_with_progress(download_url, &file_path).await?;

        println!("📦 正在解压资源包...");

        // 创建安装目录
        let install_dir = dirs::home_dir()
            .ok_or("无法获取用户主目录")?
            .join(".fnva")
            .join("java-packages")
            .join(format!("jdk-{}", version_info.version));

        fs::create_dir_all(&install_dir)
            .map_err(|e| format!("创建安装目录失败: {}", e))?;

        // 解压文件
        if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
            Self::extract_tar_gz(&file_path, &install_dir)?;
        } else if file_name.ends_with(".zip") {
            Self::extract_zip(&file_path, &install_dir)?;
        } else {
            return Err(format!("不支持的资源包格式: {}", file_name));
        }

        // 查找实际的 JAVA_HOME
        let java_home = Self::find_java_home_in_package(&install_dir)?;
        Ok(java_home)
    }

    /// 从 URL 提取文件名
    fn extract_filename_from_url(url: &str) -> String {
        url.split('/')
            .last()
            .unwrap_or("java-package")
            .to_string()
    }

    /// 下载文件并显示进度
    async fn download_file_with_progress(url: &str, dest_path: &Path) -> Result<(), String> {
        let max_retries = 3;
        let retry_delay = std::time::Duration::from_secs(2);

        for attempt in 1..=max_retries {
            println!("📥 尝试下载资源包 (第 {} 次)...", attempt);

            match Self::download_attempt(url, dest_path).await {
                Ok(()) => {
                    println!("✅ 资源包下载成功完成");
                    return Ok(());
                }
                Err(e) => {
                    println!("⚠️  下载失败 (第 {} 次): {}", attempt, e);

                    if attempt < max_retries {
                        println!("⏳ {} 秒后重试...", retry_delay.as_secs());
                        tokio::time::sleep(retry_delay).await;
                    } else {
                        return Err(format!("资源包下载失败，已重试 {} 次: {}", max_retries, e));
                    }
                }
            }
        }

        Err("资源包下载失败".to_string())
    }

    /// 单次下载尝试
    async fn download_attempt(url: &str, dest_path: &Path) -> Result<(), String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600)) // 10分钟超时
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

        println!("🔗 正在连接: {}", url);

        let response = client
            .get(url)
            .header("User-Agent", "fnva/0.0.4")
            .send()
            .await
            .map_err(|e| format!("下载请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("服务器返回错误: {} {}", response.status(), response.status().canonical_reason().unwrap_or("Unknown")));
        }

        let total_size = response.content_length().unwrap_or(0);
        println!("📊 资源包大小: {} MB", total_size / (1024 * 1024));

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta}) {percent}%")
                .unwrap()
                .progress_chars("#>-")
        );

        let mut file = File::create(dest_path)
            .await
            .map_err(|e| format!("创建文件失败: {}", e))?;

        let mut downloaded = 0u64;
        let mut stream = response.bytes_stream();

        while let Some(item) = stream.next().await {
            let chunk = item.map_err(|e| {
                if e.is_timeout() {
                    "下载超时，请检查网络连接".to_string()
                } else if e.is_connect() {
                    "连接失败，请检查网络设置".to_string()
                } else {
                    format!("下载流错误: {}", e)
                }
            })?;

            file.write_all(&chunk)
                .await
                .map_err(|e| format!("写入文件失败: {}", e))?;

            let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
            downloaded = new;
            pb.set_position(new);
        }

        pb.finish_with_message("资源包下载完成");
        file.flush().await
            .map_err(|e| format!("刷新文件失败: {}", e))?;

        // 验证文件大小
        let metadata = tokio::fs::metadata(dest_path).await
            .map_err(|e| format!("获取文件信息失败: {}", e))?;

        if total_size > 0 && metadata.len() != total_size {
            return Err(format!("文件大小不匹配: 期望 {} 字节，实际 {} 字节", total_size, metadata.len()));
        }

        Ok(())
    }

    /// 解压 TAR.GZ 文件
    fn extract_tar_gz(tar_path: &Path, dest_dir: &Path) -> Result<(), String> {
        println!("📂 解压 TAR.GZ 文件...");

        #[cfg(unix)]
        {
            use std::process::Command;
            let output = Command::new("tar")
                .args([
                    "-xzf", tar_path.to_str().unwrap(),
                    "-C", dest_dir.to_str().unwrap(),
                    "--strip-components=1"
                ])
                .output()
                .map_err(|e| format!("执行解压命令失败: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("解压失败: {}", stderr));
            }
        }

        #[cfg(not(unix))]
        {
            // Windows 平台尝试使用内置解压或其他工具
            if cfg!(target_os = "windows") {
                // 对于 Windows，我们优先使用 ZIP 格式
                return Err("Windows 平台建议使用 ZIP 格式的资源包".to_string());
            }
        }

        Ok(())
    }

    /// 解压 ZIP 文件
    fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
        println!("📂 解压 ZIP 文件...");

        let file = fs::File::open(zip_path)
            .map_err(|e| format!("打开 ZIP 文件失败: {}", e))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("读取 ZIP 文件失败: {}", e))?;

        // 检测是否需要去除第一层目录
        let mut strip_components = 0;
        if archive.len() > 3 {
            // 读取前几个条目来检测目录结构
            let sample_size = std::cmp::min(10, archive.len());
            let mut first_dirs = Vec::new();

            for i in 0..sample_size {
                let file_name = {
                    let file = archive.by_index(i)
                        .map_err(|e| format!("读取文件项失败: {}", e))?;
                    let name = file.name().to_string();
                    drop(file); // 立即释放借用
                    name
                };

                let parts: Vec<&str> = file_name.split('/').collect();
                if parts.len() > 1 && parts[0].contains("jdk") {
                    first_dirs.push(parts[0].to_string());
                }
            }

            // 如果检测到一致的 JDK 目录前缀，则去除
            if let Some(first_dir) = first_dirs.first() {
                let all_same = first_dirs.iter().all(|dir| dir == first_dir);
                if all_same && !first_dir.is_empty() {
                    strip_components = 1;
                    println!("🔧 检测到 JDK 目录层级，自动去除: {}", first_dir);
                }
            }
        }

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("读取 ZIP 文件项失败: {}", e))?;

            let file_path = file.mangled_name();
            let mut final_path = file_path.clone();

            // 去除指定数量的目录层级
            if strip_components > 0 {
                let components: Vec<std::path::Component> = file_path.components().collect();
                if components.len() > strip_components {
                    let mut new_path = std::path::PathBuf::new();
                    for component in components.iter().skip(strip_components) {
                        new_path.push(component);
                    }
                    final_path = new_path;
                } else {
                    // 跳过根级别的目录文件
                    continue;
                }
            }

            // 跳过空路径（根目录）
            if final_path == std::path::PathBuf::new() {
                continue;
            }

            let outpath = dest_dir.join(&final_path);

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)
                    .map_err(|e| format!("创建目录失败: {}", e))?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)
                            .map_err(|e| format!("创建父目录失败: {}", e))?;
                    }
                }

                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| format!("创建文件失败: {}", e))?;

                std::io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("写入文件失败: {}", e))?;
            }
        }

        println!("✅ ZIP 文件解压完成");
        Ok(())
    }

    /// 在资源包中查找 JAVA_HOME
    fn find_java_home_in_package(package_dir: &Path) -> Result<String, String> {
        println!("🔍 在资源包中查找 Java 安装目录...");

        // 常见的 Java 目录结构
        let search_paths = vec![
            package_dir.to_path_buf(),
            package_dir.join("jdk"),
            package_dir.join("jre"),
            package_dir.join("java"),
        ];

        // 检查每个可能的路径
        for search_path in search_paths {
            if validate_java_home(&search_path.to_string_lossy()) {
                println!("✅ 找到 Java 安装目录: {}", search_path.display());
                return Ok(search_path.to_string_lossy().to_string());
            }

            // 检查子目录
            if search_path.is_dir() {
                if let Ok(entries) = fs::read_dir(&search_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && validate_java_home(&path.to_string_lossy()) {
                            println!("✅ 找到 Java 安装目录: {}", path.display());
                            return Ok(path.to_string_lossy().to_string());
                        }
                    }
                }
            }

            // 对于 macOS，检查 Contents/Home
            if cfg!(target_os = "macos") {
                let contents_home = search_path.join("Contents").join("Home");
                if contents_home.exists() && validate_java_home(&contents_home.to_string_lossy()) {
                    println!("✅ 找到 Java 安装目录: {}", contents_home.display());
                    return Ok(contents_home.to_string_lossy().to_string());
                }
            }
        }

        Err("在资源包中未找到有效的 Java 安装目录".to_string())
    }

    /// 切换到指定的 Java 版本
    fn switch_to_java(version_name: &str, config: &Config) -> Result<(), String> {
        let java_env = config.get_java_env(version_name)
            .ok_or_else(|| format!("Java 环境 '{}' 不存在", version_name))?;

        // 验证 Java Home 路径
        if !validate_java_home(&java_env.java_home) {
            return Err(format!("无效的 JAVA_HOME 路径: {}", java_env.java_home));
        }

        println!("🔄 切换到 Java: {} ({})", version_name, java_env.java_home);
        println!("💡 请在新的终端中运行以下命令来激活环境:");
        println!("   fnva java use {}", version_name);

        Ok(())
    }

    /// 列出可安装的资源包版本
    pub async fn list_installable_packages() -> Result<Vec<String>, String> {
        let mut packages = Vec::new();

        // 加载配置以获取仓库列表
        let config = Config::load().map_err(|e| format!("加载配置失败: {}", e))?;
        let repositories = &config.repositories.java.repositories;

        for major_version in [21, 17, 11, 8] {
            let mut found = false;

            for repo in repositories {
                let mut remote_manager = RemoteManager::new();
                match remote_manager.list_java_versions(
                    Some(repo),
                    Some(major_version),
                    None,
                    None,
                ).await {
                    Ok(mut version_list) => {
                        if let Some(version) = version_list.pop() {
                            packages.push(format!("v{} ({} - Portable Package)", major_version, version.version));
                            found = true;
                            break; // 找到就停止尝试其他仓库
                        }
                    }
                    Err(_) => {
                        // 尝试下一个仓库
                        continue;
                    }
                }
            }

            if !found {
                packages.push(format!("v{} (Portable Package - 查询失败)", major_version));
            }
        }

        Ok(packages)
    }

    /// 卸载 Java 资源包
    pub fn uninstall_java_package(package_name: &str, config: &mut Config) -> Result<(), String> {
        let java_env = config.get_java_env(package_name)
            .ok_or_else(|| format!("Java 资源包 '{}' 不存在", package_name))?;

        let java_home = &java_env.java_home;

        // 检查是否是 fnva 管理的资源包
        if !java_home.contains(".fnva/java-packages") {
            return Err("只能卸载通过 fnva 安装的 Java 资源包".to_string());
        }

        println!("🗑️  正在卸载 Java 资源包 {}...", package_name);
        println!("📁 删除路径: {}", java_home);

        // 删除安装目录
        fs::remove_dir_all(java_home)
            .map_err(|e| format!("删除安装目录失败: {}", e))?;

        // 从配置中移除
        config.remove_java_env(package_name)?;
        config.save()?;

        println!("✅ Java 资源包 {} 卸载成功", package_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_spec() {
        assert_eq!(JavaPackageManager::parse_version_spec("v21").unwrap(), 21);
        assert_eq!(JavaPackageManager::parse_version_spec("21").unwrap(), 21);
        assert_eq!(JavaPackageManager::parse_version_spec("jdk21").unwrap(), 21);
        assert_eq!(JavaPackageManager::parse_version_spec("pkg21").unwrap(), 21);
        assert_eq!(JavaPackageManager::parse_version_spec("V11").unwrap(), 11);

        assert!(JavaPackageManager::parse_version_spec("22").is_err());
        assert!(JavaPackageManager::parse_version_spec("invalid").is_err());
    }

    #[test]
    fn test_detect_platform_info() {
        let (version, arch, os) = JavaPackageManager::detect_platform_info();
        assert!(!version.is_empty());
        assert!(!arch.is_empty());
        assert!(!os.is_empty());
    }
}