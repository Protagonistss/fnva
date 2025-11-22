use crate::utils::validation::ValidationUtils;
use std::path::Path;

/// Java 环境验证器
pub struct JavaValidator;

impl JavaValidator {
    /// 验证 Java 环境
    pub fn validate_environment(name: &str, java_home: &str) -> Result<(), String> {
        // 验证环境名称
        ValidationUtils::validate_environment_name(name)?;

        // 验证 Java Home 路径
        if !ValidationUtils::validate_java_home(java_home) {
            return Err(format!("Invalid JAVA_HOME path: {}", java_home));
        }

        // 检查版本信息
        Self::validate_java_version(java_home)?;

        Ok(())
    }

    /// 验证 Java 版本是否可获取
    fn validate_java_version(java_home: &str) -> Result<(), String> {
        let java_exe = if cfg!(target_os = "windows") {
            format!("{}\\bin\\java.exe", java_home)
        } else {
            format!("{}/bin/java", java_home)
        };

        let java_path = Path::new(&java_exe);
        if !java_path.exists() {
            return Err(format!("Java executable not found: {}", java_exe));
        }

        use std::process::Command;
        let output = Command::new(java_exe)
            .arg("-version")
            .output()
            .map_err(|e| format!("Failed to execute java -version: {}", e))?;

        if !output.status.success() {
            return Err(format!("java -version command failed: {}", output.status));
        }

        Ok(())
    }

    /// 验证 Java 版本格式
    pub fn validate_version_format(version: &str) -> Result<(), String> {
        ValidationUtils::validate_version(version)
    }

    /// 验证 Java 供应商
    pub fn validate_vendor(vendor: &str) -> Result<(), String> {
        if vendor.is_empty() {
            return Ok(()); // 供应商是可选的
        }

        // 常见供应商列表
        let valid_vendors = [
            "Oracle",
            "Eclipse Adoptium",
            "Amazon",
            "Microsoft",
            "Azul Zulu",
            "BellSoft Liberica",
            "OpenLogic",
            "Red Hat",
            "IBM",
            "SAP",
            "AdoptOpenJDK",
            "Corretto",
            "Zulu",
            "Liberica",
            "Temurin",
        ];

        for valid_vendor in &valid_vendors {
            if vendor.to_lowercase().contains(&valid_vendor.to_lowercase()) {
                return Ok(());
            }
        }

        // 如果不在已知列表中，给出警告但不阻止
        eprintln!("Warning: Unknown Java vendor: {}", vendor);
        Ok(())
    }
}
