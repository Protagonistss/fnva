use serde::{Serialize, Deserialize};
use crate::core::environment_manager::EnvironmentInfo;

/// Java 安装信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaInstallation {
    pub name: String,
    pub description: String,
    pub java_home: String,
    pub version: Option<String>,
    pub vendor: Option<String>,
}

impl EnvironmentInfo for JavaInstallation {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn is_active(&self) -> bool {
        // 检查是否是当前激活的环境
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            // 标准化两个路径进行比较
            let current_home = std::path::Path::new(&java_home)
                .canonicalize()
                .unwrap_or_else(|_| java_home.into())
                .to_string_lossy()
                .to_lowercase();

            let install_home = std::path::Path::new(&self.java_home)
                .canonicalize()
                .unwrap_or_else(|_| self.java_home.clone().into())
                .to_string_lossy()
                .to_lowercase();

            current_home == install_home
        } else {
            false
        }
    }

    fn get_identifier(&self) -> &str {
        &self.java_home
    }
}

/// Java 环境扫描器
pub struct JavaScanner;

impl JavaScanner {
    /// 扫描系统中的 Java 安装
    pub fn scan_system() -> Result<Vec<JavaInstallation>, String> {
        let mut installations = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        // 扫描常见路径
        let common_paths = Self::get_common_java_paths();

        for path in common_paths {
            // 首先尝试直接路径
            if Self::is_valid_java_installation(&path) {
                let normalized_path = Self::normalize_path(&path);
                if !seen_paths.contains(&normalized_path) {
                    if let Ok(installation) = Self::create_installation_from_path(&path) {
                        installations.push(installation);
                        seen_paths.insert(normalized_path);
                    }
                }
            } else {
                // 如果直接路径无效，尝试扫描子目录
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.is_dir() {
                            let path_str = entry_path.to_string_lossy();
                            let normalized_path = Self::normalize_path(&path_str);
                            if !seen_paths.contains(&normalized_path) && Self::is_valid_java_installation(&path_str) {
                                if let Ok(installation) = Self::create_installation_from_path(&path_str) {
                                    installations.push(installation);
                                    seen_paths.insert(normalized_path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 扫描 PATH 中的 Java
        if let Ok(Some(path_java)) = Self::scan_path_java() {
            let normalized_path = Self::normalize_path(&path_java.java_home);
            if !seen_paths.contains(&normalized_path) {
                installations.push(path_java);
            }
        }

        Ok(installations)
    }

    /// 标准化路径格式，处理反斜杠和大小写问题
    fn normalize_path(path: &str) -> String {
        use std::path::Path;

        // 转换为 Path 对象来标准化路径分隔符
        let path = Path::new(path);

        // 获取规范化路径
        match path.canonicalize() {
            Ok(canonical_path) => {
                // 转换回字符串，保持原始格式
                canonical_path.to_string_lossy().to_string()
            }
            Err(_) => {
                // 如果无法规范化，至少标准化分隔符
                path.to_string_lossy()
                    .replace('\\', "/")
                    .to_lowercase()
            }
        }
    }

    /// 获取常见的 Java 安装路径
    fn get_common_java_paths() -> Vec<String> {
        let mut paths = Vec::new();

        if cfg!(target_os = "windows") {
            // Windows 常见路径
            paths.extend_from_slice(&[
                r"C:\Program Files\Java".to_string(),
                r"C:\Program Files (x86)\Java".to_string(),
                r"C:\Program Files\Eclipse Adoptium".to_string(),
                r"C:\Program Files\Amazon Corretto".to_string(),
                r"C:\Program Files\Microsoft\jdk".to_string(),
                r"C:\Program Files\Zulu".to_string(),
            ]);

            // 动态添加用户相关的路径
            if let Some(home_dir) = dirs::home_dir() {
                let home_str = home_dir.to_string_lossy();
                paths.push(format!("{}\\.fnva\\java-packages", home_str));
            }

            // 从配置文件读取自定义路径（如果存在）
            if let Ok(custom_paths) = Self::get_custom_scan_paths() {
                paths.extend(custom_paths);
            }
        } else if cfg!(target_os = "macos") {
            // macOS 常见路径
            paths.extend_from_slice(&[
                "/Library/Java/JavaVirtualMachines".to_string(),
                "/System/Library/Java/JavaVirtualMachines".to_string(),
                "/usr/local/java".to_string(),
                "/opt/homebrew/Caskroom".to_string(),
            ]);

            // 动态添加用户相关的路径
            if let Some(home_dir) = dirs::home_dir() {
                let home_str = home_dir.to_string_lossy();
                paths.push(format!("{}/.fnva/java-packages", home_str));
            }

            // 从配置文件读取自定义路径
            if let Ok(custom_paths) = Self::get_custom_scan_paths() {
                paths.extend(custom_paths);
            }
        } else {
            // Linux 常见路径
            paths.extend_from_slice(&[
                "/usr/lib/jvm".to_string(),
                "/usr/lib/jvm/java".to_string(),
                "/usr/local/java".to_string(),
                "/opt/java".to_string(),
                "/usr/java".to_string(),
            ]);

            // 动态添加用户相关的路径
            if let Some(home_dir) = dirs::home_dir() {
                let home_str = home_dir.to_string_lossy();
                paths.push(format!("{}/.fnva/java-packages", home_str));
            }

            // 从配置文件读取自定义路径
            if let Ok(custom_paths) = Self::get_custom_scan_paths() {
                paths.extend(custom_paths);
            }
        }

        paths
    }

    /// 从配置文件获取自定义扫描路径
    fn get_custom_scan_paths() -> Result<Vec<String>, String> {
        use crate::infrastructure::config::Config;

        let config = Config::load().map_err(|e| format!("Failed to load config: {}", e))?;

        let mut custom_paths = Vec::new();

        // 从配置文件的自定义扫描路径读取
        for path in &config.custom_java_scan_paths {
            if !path.trim().is_empty() {
                custom_paths.push(path.trim().to_string());
            }
        }

        // 从环境变量读取额外路径
        if let Ok(env_paths) = std::env::var("FNVA_SCAN_PATHS") {
            for path in env_paths.split(';') {
                if !path.trim().is_empty() {
                    custom_paths.push(path.trim().to_string());
                }
            }
        }

        Ok(custom_paths)
    }

    /// 检查路径是否是有效的 Java 安装
    pub fn is_valid_java_installation(path: &str) -> bool {
        let java_home = std::path::Path::new(path);

        // 检查 bin 目录
        let bin_dir = java_home.join("bin");
        if !bin_dir.exists() {
            return false;
        }

        // 检查 java 可执行文件
        let java_exe = if cfg!(target_os = "windows") {
            bin_dir.join("java.exe")
        } else {
            bin_dir.join("java")
        };

        java_exe.exists() && java_exe.is_file()
    }

    /// 从路径创建 Java 安装信息
    pub fn create_installation_from_path(path: &str) -> Result<JavaInstallation, String> {
        let _java_home = std::path::Path::new(path);
        let name = Self::extract_name_from_path(path)?;
        let version = Self::detect_java_version(path)?;
        let vendor = Self::detect_vendor(path)?;

        let installation = JavaInstallation {
            name: name.clone(),
            description: format!("Java {} ({})",
                version.as_deref().unwrap_or("unknown"),
                path),
            java_home: path.to_string(),
            version,
            vendor,
        };

        Ok(installation)
    }

    /// 从路径提取名称
    fn extract_name_from_path(path: &str) -> Result<String, String> {
        let java_home = std::path::Path::new(path);

        if let Some(dir_name) = java_home.file_name() {
            if let Some(name_str) = dir_name.to_str() {
                let mut name = name_str.to_string();

                // 清理名称
                name = name.replace("jdk-", "jdk");
                name = name.replace("jre-", "jre");

                return Ok(name);
            }
        }

        Err("Could not extract name from path".to_string())
    }

    /// 检测 Java 版本
    fn detect_java_version(path: &str) -> Result<Option<String>, String> {
        let java_home = std::path::Path::new(path);
        let java_exe = if cfg!(target_os = "windows") {
            java_home.join("bin/java.exe")
        } else {
            java_home.join("bin/java")
        };

        if !java_exe.exists() {
            return Ok(None);
        }

        use std::process::Command;
        let output = Command::new(java_exe)
            .arg("-version")
            .output()
            .map_err(|e| format!("Failed to execute java -version: {}", e))?;

        if output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let lines: Vec<&str> = stderr.lines().collect();
            if let Some(first_line) = lines.first() {
                // 解析版本信息，例如："openjdk version "17.0.2" 2022-01-18"
                if let Some(start) = first_line.find('"') {
                    if let Some(end) = first_line.rfind('"') {
                        let version = &first_line[start + 1..end];
                        return Ok(Some(version.to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 检测供应商信息
    fn detect_vendor(path: &str) -> Result<Option<String>, String> {
        let path_lower = path.to_lowercase();

        if path_lower.contains("adoptium") || path_lower.contains("adoptopenjdk") {
            Ok(Some("Eclipse Adoptium".to_string()))
        } else if path_lower.contains("amazon") || path_lower.contains("corretto") {
            Ok(Some("Amazon".to_string()))
        } else if path_lower.contains("microsoft") {
            Ok(Some("Microsoft".to_string()))
        } else if path_lower.contains("oracle") {
            Ok(Some("Oracle".to_string()))
        } else if path_lower.contains("openlogic") {
            Ok(Some("OpenLogic".to_string()))
        } else if path_lower.contains("zulu") {
            Ok(Some("Azul Zulu".to_string()))
        } else if path_lower.contains("liberica") {
            Ok(Some("BellSoft Liberica".to_string()))
        } else {
            Ok(None)
        }
    }

    /// 扫描 PATH 中的 Java
    fn scan_path_java() -> Result<Option<JavaInstallation>, String> {
        use std::env;

        if let Ok(path_var) = env::var("PATH") {
            let path_separator = if cfg!(target_os = "windows") { ';' } else { ':' };

            for path_dir in path_var.split(path_separator) {
                let java_exe = if cfg!(target_os = "windows") {
                    std::path::Path::new(path_dir).join("java.exe")
                } else {
                    std::path::Path::new(path_dir).join("java")
                };

                if java_exe.exists() && java_exe.is_file() {
                    // 找到 Java，尝试确定 JAVA_HOME
                    if let Some(java_home) = java_exe.parent().and_then(|p| p.parent()) {
                        if Self::is_valid_java_installation(java_home.to_str().unwrap_or("")) {
                            return Ok(Some(Self::create_installation_from_path(java_home.to_str().unwrap_or(""))?));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_vendor() {
        assert_eq!(
            JavaScanner::detect_vendor("/usr/lib/jvm/adoptopenjdk-11").unwrap(),
            Some("Eclipse Adoptium".to_string())
        );

        assert_eq!(
            JavaScanner::detect_vendor("C:\\Program Files\\Amazon Corretto\\jdk17").unwrap(),
            Some("Amazon".to_string())
        );
    }

    #[test]
    fn test_extract_name_from_path() {
        assert_eq!(
            JavaScanner::extract_name_from_path("/usr/lib/jvm/java-11-openjdk").unwrap(),
            "java-11-openjdk"
        );

        assert_eq!(
            JavaScanner::extract_name_from_path("C:\\Program Files\\Java\\jdk-17").unwrap(),
            "jdk17"
        );
    }
}