use crate::config::Config;
use crate::remote::{JavaVersionInfo, RemoteManager};
use crate::utils::validate_java_home;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Java 安装管理器
pub struct JavaInstaller;

impl JavaInstaller {
    /// 安装指定版本的 Java
    pub async fn install_java(
        version_spec: &str,
        config: &mut Config,
        auto_switch: bool,
    ) -> Result<String, String> {
        println!("正在准备安装 Java {}...", version_spec);

        // 解析版本规格
        let java_version = Self::parse_version_spec(version_spec)?;

        // 获取版本信息
        let version_info = Self::get_version_info(&java_version).await?;

        // 检查是否已安装
        let install_name = format!("jdk-{}", version_info.version);
        if config.get_java_env(&install_name).is_some() {
            return Err(format!("Java {} 已经安装", version_info.version));
        }

        // 下载和安装
        let java_home = Self::download_and_install(&version_info).await?;

        // 添加到配置
        let description = format!("Auto-installed Java {}", version_info.version);
        config.add_java_env(crate::config::JavaEnvironment {
            name: install_name.clone(),
            java_home: java_home.clone(),
            description,
            source: crate::config::EnvironmentSource::Manual,
        })?;
        config.save()?;

        println!("✅ Java {} 安装成功！", version_info.version);
        println!("📁 安装路径: {}", java_home);

        // 自动切换
        if auto_switch {
            println!("🔄 自动切换到 Java {}", version_info.version);
            if let Err(e) = Self::switch_to_java(&install_name, config) {
                println!("⚠️  自动切换失败: {}", e);
            } else {
                println!("✅ 已切换到 Java {}", version_info.version);
            }
        }

        Ok(java_home)
    }

    /// 解析版本规格
    fn parse_version_spec(version_spec: &str) -> Result<u32, String> {
        // 支持格式: "v21", "21", "java21", "jdk21" 等
        let cleaned = version_spec
            .trim()
            .to_lowercase()
            .replace("v", "")
            .replace("java", "")
            .replace("jdk", "");

        if let Ok(version) = cleaned.parse::<u32>() {
            // 验证支持的版本
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

    /// 获取版本信息，支持备用源
    async fn get_version_info(major_version: &u32) -> Result<JavaVersionInfo, String> {
        // 尝试多个源
        let repositories = vec![
            "https://api.adoptium.net/v3",
            "https://api.adoptopenjdk.net/v3",
        ];

        for repo in repositories {
            println!("🔍 尝试从 {} 获取版本信息...", repo);

            match RemoteManager::list_java_versions(
                repo,
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

    /// 下载和安装 Java
    async fn download_and_install(version_info: &JavaVersionInfo) -> Result<String, String> {
        let download_url = version_info.download_url.as_ref()
            .ok_or("没有可用的下载链接")?;

        println!("📥 正在下载 Java {}...", version_info.version);
        println!("🔗 下载地址: {}", download_url);

        // 创建临时目录
        let temp_dir = TempDir::new()
            .map_err(|e| format!("创建临时目录失败: {}", e))?;

        let file_name = Self::extract_filename_from_url(download_url);
        let file_path = temp_dir.path().join(&file_name);

        // 下载文件
        Self::download_file_with_progress(download_url, &file_path).await?;

        println!("📦 正在安装...");

        // 根据文件类型进行安装
        let java_home = if file_name.ends_with(".msi") {
            Self::install_msi(&file_path, &version_info.version).await?
        } else if file_name.ends_with(".zip") || file_name.ends_with(".tar.gz") {
            Self::install_archive(&file_path, &version_info.version).await?
        } else {
            return Err(format!("不支持的安装包格式: {}", file_name));
        };

        // 验证安装
        if !validate_java_home(&java_home) {
            return Err("安装验证失败".to_string());
        }

        Ok(java_home)
    }

    /// 从 URL 提取文件名
    fn extract_filename_from_url(url: &str) -> String {
        url.split('/')
            .last()
            .unwrap_or("java-installer")
            .to_string()
    }

    /// 下载文件并显示进度，带重试机制
    async fn download_file_with_progress(url: &str, dest_path: &Path) -> Result<(), String> {
        let max_retries = 3;
        let retry_delay = std::time::Duration::from_secs(2);

        for attempt in 1..=max_retries {
            println!("📥 尝试下载 (第 {} 次)...", attempt);

            match Self::download_attempt(url, dest_path).await {
                Ok(()) => {
                    println!("✅ 下载成功完成");
                    return Ok(());
                }
                Err(e) => {
                    println!("⚠️  下载失败 (第 {} 次): {}", attempt, e);

                    if attempt < max_retries {
                        println!("⏳ {} 秒后重试...", retry_delay.as_secs());
                        tokio::time::sleep(retry_delay).await;
                    } else {
                        return Err(format!("下载失败，已重试 {} 次: {}", max_retries, e));
                    }
                }
            }
        }

        Err("下载失败".to_string())
    }

    /// 单次下载尝试
    async fn download_attempt(url: &str, dest_path: &Path) -> Result<(), String> {
        // 网络连接诊断
        Self::diagnose_network_connection(url).await?;

        // 创建客户端，设置超时和重试
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5分钟超时
            .connect_timeout(std::time::Duration::from_secs(30)) // 连接超时30秒
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

        let total_size = response.content_length()
            .unwrap_or(0);

        println!("📊 文件大小: {} MB", total_size / (1024 * 1024));

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
                // 提供更详细的错误信息
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

        pb.finish_with_message("下载完成");
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

    /// 网络连接诊断
    async fn diagnose_network_connection(url: &str) -> Result<(), String> {
        println!("🔍 诊断网络连接...");

        // 解析 URL
        let parsed_url = url::Url::parse(url)
            .map_err(|e| format!("无效的 URL: {}", e))?;

        let host = parsed_url.host_str()
            .ok_or("无法解析主机名")?;

        println!("🌐 主机: {}", host);
        println!("🔍 测试连接...");

        // 测试 DNS 解析
        match tokio::net::lookup_host(format!("{}:80", host)).await {
            Ok(addresses) => {
                let addr_vec: Vec<_> = addresses.collect();
                if addr_vec.is_empty() {
                    return Err("DNS 解析失败：没有找到地址".to_string());
                }
                println!("✅ DNS 解析成功: {:?}", addr_vec.first());
            }
            Err(e) => {
                return Err(format!("DNS 解析失败: {}", e));
            }
        }

        // 测试 HTTPS 连接
        match tokio::net::TcpStream::connect(format!("{}:443", host)).await {
            Ok(_) => {
                println!("✅ TCP 连接成功");
            }
            Err(e) => {
                return Err(format!("TCP 连接失败: {}。可能的原因：防火墙阻止、网络不可达或服务器不可用", e));
            }
        }

        Ok(())
    }

    /// 安装 MSI 文件（Windows）
    async fn install_msi(msi_path: &Path, version: &str) -> Result<String, String> {
        if cfg!(target_os = "windows") {
            // 获取 fnva 安装目录
            let fnva_dir = dirs::home_dir()
                .ok_or("无法获取用户主目录")?
                .join(".fnva")
                .join("java");

            fs::create_dir_all(&fnva_dir)
                .map_err(|e| format!("创建安装目录失败: {}", e))?;

            let java_home = fnva_dir.join(format!("jdk-{}", version));

            // 使用 msiexec 静默安装到指定目录
            let output = Command::new("msiexec")
                .args([
                    "/i", msi_path.to_str().unwrap(),
                    "/quiet",
                    &format!("INSTALLDIR={}", java_home.to_str().unwrap()),
                    "ADDLOCAL=FeatureMain,FeatureEnvironment,FeatureJarFileRunWith,FeatureJavaHome",
                    "INSTALLDIR2={}",
                ])
                .output()
                .map_err(|e| format!("执行安装命令失败: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("MSI 安装失败: {}", stderr));
            }

            // 等待安装完成
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            // 查找实际的 JAVA_HOME
            let actual_home = Self::find_installed_java(&java_home)?;
            Ok(actual_home)
        } else {
            Err("MSI 安装包仅支持 Windows".to_string())
        }
    }

    /// 安装压缩包（跨平台）
    async fn install_archive(archive_path: &Path, version: &str) -> Result<String, String> {
        // 获取 fnva 安装目录
        let fnva_dir = dirs::home_dir()
            .ok_or("无法获取用户主目录")?
            .join(".fnva")
            .join("java");

        fs::create_dir_all(&fnva_dir)
            .map_err(|e| format!("创建安装目录失败: {}", e))?;

        let java_home = fnva_dir.join(format!("jdk-{}", version));

        // 解压文件
        if archive_path.to_str().unwrap().ends_with(".zip") {
            Self::extract_zip(archive_path, &java_home)?;
        } else {
            Self::extract_tar_gz(archive_path, &java_home)?;
        }

        // 查找实际的 JAVA_HOME（可能在子目录中）
        let actual_home = Self::find_installed_java(&java_home)?;
        Ok(actual_home)
    }

    /// 解压 ZIP 文件
    fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
        let file = fs::File::open(zip_path)
            .map_err(|e| format!("打开 ZIP 文件失败: {}", e))?;

        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| format!("读取 ZIP 文件失败: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| format!("读取 ZIP 文件项失败: {}", e))?;

            let outpath = dest_dir.join(file.mangled_name());

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

        Ok(())
    }

    /// 解压 tar.gz 文件
    fn extract_tar_gz(tar_path: &Path, dest_dir: &Path) -> Result<(), String> {
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

        Ok(())
    }

    /// 查找已安装的 Java 目录
    fn find_installed_java(install_dir: &Path) -> Result<String, String> {
        // 检查是否直接包含 Java 安装
        if validate_java_home(&install_dir.to_string_lossy()) {
            return Ok(install_dir.to_string_lossy().to_string());
        }

        // 搜索子目录
        for entry in fs::read_dir(install_dir)
            .map_err(|e| format!("读取安装目录失败: {}", e))?
        {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();

            if path.is_dir() && validate_java_home(&path.to_string_lossy()) {
                return Ok(path.to_string_lossy().to_string());
            }

            // 对于 macOS，检查 Contents/Home
            if cfg!(target_os = "macos") {
                let contents_home = path.join("Contents").join("Home");
                if contents_home.exists() && validate_java_home(&contents_home.to_string_lossy()) {
                    return Ok(contents_home.to_string_lossy().to_string());
                }
            }
        }

        Err("未找到有效的 Java 安装目录".to_string())
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

    /// 列出可安装的 Java 版本
    pub async fn list_installable_versions() -> Result<Vec<String>, String> {
        let mut versions = Vec::new();

        for major_version in [21, 17, 11, 8] {
            match RemoteManager::list_java_versions(
                "https://api.adoptium.net/v3",
                Some(major_version),
                None,
                None,
            ).await {
                Ok(mut version_list) => {
                    if let Some(version) = version_list.pop() {
                        versions.push(format!("v{} ({})", major_version, version.version));
                    }
                }
                Err(_) => {
                    versions.push(format!("v{} (查询失败)", major_version));
                }
            }
        }

        Ok(versions)
    }

    /// 卸载 Java 版本
    pub fn uninstall_java(version_name: &str, config: &mut Config) -> Result<(), String> {
        let java_env = config.get_java_env(version_name)
            .ok_or_else(|| format!("Java 环境 '{}' 不存在", version_name))?;

        let java_home = &java_env.java_home;

        // 检查是否是 fnva 管理的安装
        if !java_home.contains(".fnva/java") {
            return Err("只能卸载通过 fnva 安装的 Java 版本".to_string());
        }

        println!("🗑️  正在卸载 Java {}...", version_name);
        println!("📁 删除路径: {}", java_home);

        // 删除安装目录
        fs::remove_dir_all(java_home)
            .map_err(|e| format!("删除安装目录失败: {}", e))?;

        // 从配置中移除
        config.remove_java_env(version_name)?;
        config.save()?;

        println!("✅ Java {} 卸载成功", version_name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_spec() {
        assert_eq!(JavaInstaller::parse_version_spec("v21").unwrap(), 21);
        assert_eq!(JavaInstaller::parse_version_spec("21").unwrap(), 21);
        assert_eq!(JavaInstaller::parse_version_spec("java21").unwrap(), 21);
        assert_eq!(JavaInstaller::parse_version_spec("jdk21").unwrap(), 21);
        assert_eq!(JavaInstaller::parse_version_spec("V11").unwrap(), 11);

        assert!(JavaInstaller::parse_version_spec("22").is_err());
        assert!(JavaInstaller::parse_version_spec("invalid").is_err());
    }
}