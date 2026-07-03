use crate::core::environment_manager::EnvironmentInfo;
use crate::core::presentation::ScanHit;
use serde::{Deserialize, Serialize};

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
    pub async fn scan_system(extra: &[String]) -> Result<Vec<ScanHit>, String> {
        use crate::infrastructure::scanner::scan_directory_roots;
        let extra_owned = extra.to_vec();
        tokio::task::spawn_blocking(move || {
            let roots = crate::environments::java::paths::common_paths(&extra_owned);
            let hits = scan_directory_roots(
                &roots,
                |p: &std::path::Path| Self::is_valid_java_installation(&p.to_string_lossy()),
                |p: &std::path::Path| {
                    let java_home = p.to_string_lossy().to_string();
                    let name = Self::extract_name_from_path(&java_home)?;
                    let version = Self::detect_java_version(&java_home).ok().flatten();
                    let detail = version.unwrap_or_else(|| "unknown".to_string());
                    let import = format!("fnva java add --name {name} --home \"{java_home}\"");
                    Ok(ScanHit {
                        name,
                        location: java_home,
                        detail,
                        import_cmd: import,
                    })
                },
            );
            Ok(hits)
        })
        .await
        .map_err(|e| format!("Scanner task panicked: {e}"))?
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
            description: format!(
                "Java {} ({})",
                version.as_deref().unwrap_or("unknown"),
                path
            ),
            java_home: path.to_string(),
            version,
            vendor,
        };

        Ok(installation)
    }

    /// 从路径提取名称
    fn extract_name_from_path(path: &str) -> Result<String, String> {
        // Normalize separators so Windows-style paths (backslash) resolve the
        // final segment correctly on every platform.
        let normalized = path.replace('\\', "/");
        let java_home = std::path::Path::new(&normalized);

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
    pub fn detect_java_version(path: &str) -> Result<Option<String>, String> {
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
            .map_err(|e| format!("Failed to execute java -version: {e}"))?;

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
