use std::path::Path;

/// 验证 Java HOME 路径是否有效
pub fn validate_java_home(java_home: &str) -> bool {
    let java_path = Path::new(java_home);

    // 检查路径是否存在
    if !java_path.exists() {
        return false;
    }

    // 检查 bin 目录是否存在
    let bin_path = java_path.join("bin");
    if !bin_path.exists() {
        return false;
    }

    // 检查 java.exe 或 java 是否存在
    let java_exe = if cfg!(target_os = "windows") {
        bin_path.join("java.exe")
    } else {
        bin_path.join("java")
    };

    java_exe.exists()
}

/// 验证工具
pub struct ValidationUtils;

impl ValidationUtils {
    /// 验证 Java HOME 路径是否有效
    pub fn validate_java_home(java_home: &str) -> bool {
        let java_path = std::path::Path::new(java_home);

        // 检查路径是否存在
        if !java_path.exists() {
            return false;
        }

        // 检查 bin 目录是否存在
        let bin_path = java_path.join("bin");
        if !bin_path.exists() {
            return false;
        }

        // 检查 java.exe 或 java 是否存在
        let java_exe = if cfg!(target_os = "windows") {
            bin_path.join("java.exe")
        } else {
            bin_path.join("java")
        };

        java_exe.exists()
    }

    /// 验证环境名称是否有效
    pub fn validate_environment_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Environment name cannot be empty".to_string());
        }

        if name.len() > 100 {
            return Err("Environment name too long (max 100 characters)".to_string());
        }

        // 检查是否包含无效字符
        let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
        for &ch in &invalid_chars {
            if name.contains(ch) {
                return Err(format!("Environment name cannot contain '{ch}'"));
            }
        }

        // 检查是否以点开头
        if name.starts_with('.') {
            return Err("Environment name cannot start with a dot".to_string());
        }

        Ok(())
    }

    /// 验证 URL 是否有效
    pub fn validate_url(url: &str) -> Result<(), String> {
        if url.is_empty() {
            return Err("URL cannot be empty".to_string());
        }

        // 简单的 URL 格式检查
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("URL must start with http:// or https://".to_string());
        }

        // 使用 url crate 进行更详细的验证
        match url::Url::parse(url) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Invalid URL: {e}")),
        }
    }

    /// 验证 API Key 格式
    pub fn validate_api_key(api_key: &str) -> Result<(), String> {
        if api_key.is_empty() {
            return Err("API key cannot be empty".to_string());
        }

        // 检查是否是环境变量引用格式
        if api_key.starts_with("${") && api_key.ends_with('}') {
            let var_name = &api_key[2..api_key.len() - 1];
            if var_name.is_empty() {
                return Err("Environment variable name cannot be empty".to_string());
            }
            return Ok(());
        }

        // 检查 API key 长度（一般 API key 都比较长）
        if api_key.len() < 10 {
            return Err("API key seems too short (minimum 10 characters)".to_string());
        }

        Ok(())
    }

    /// 验证版本号格式
    pub fn validate_version(version: &str) -> Result<(), String> {
        if version.is_empty() {
            return Err("Version cannot be empty".to_string());
        }

        // 简单的版本号格式检查 (如: 1.0.0, 17, 21.0.3)
        let parts: Vec<&str> = version.split('.').collect();
        if parts.is_empty() || parts.len() > 4 {
            return Err("Invalid version format".to_string());
        }

        for part in parts {
            if part.is_empty() {
                return Err("Version part cannot be empty".to_string());
            }

            // 允许数字和一些常见的后缀
            if !part
                .chars()
                .all(|c| c.is_ascii_digit() || c == '-' || c == '_')
            {
                return Err("Version contains invalid characters".to_string());
            }
        }

        Ok(())
    }

    /// 验证文件路径
    pub fn validate_file_path(path: &str) -> Result<(), String> {
        if path.is_empty() {
            return Err("Path cannot be empty".to_string());
        }

        let path = Path::new(path);

        // 检查是否包含无效字符
        if let Some(filename) = path.file_name() {
            if let Some(name_str) = filename.to_str() {
                let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
                for &ch in &invalid_chars {
                    if name_str.contains(ch) {
                        return Err(format!("Path contains invalid character '{ch}'"));
                    }
                }
            }
        }

        Ok(())
    }

    /// 验证端口范围
    pub fn validate_port(port: u16) -> Result<(), String> {
        if port == 0 {
            return Err("Port cannot be 0".to_string());
        }

        // 保留端口范围
        if port < 1024 {
            return Err("Port should be >= 1024 (non-privileged ports)".to_string());
        }

        Ok(())
    }

    /// 验证温度参数（用于 LLM）
    pub fn validate_temperature(temperature: f64) -> Result<(), String> {
        if !(0.0..=2.0).contains(&temperature) {
            return Err("Temperature must be between 0.0 and 2.0".to_string());
        }
        Ok(())
    }

    /// 验证 max_tokens 参数
    pub fn validate_max_tokens(max_tokens: u32) -> Result<(), String> {
        if max_tokens == 0 {
            return Err("Max tokens must be greater than 0".to_string());
        }

        if max_tokens > 32768 {
            return Err("Max tokens too large (maximum 32768)".to_string());
        }

        Ok(())
    }

    /// 清理和标准化名称
    pub fn sanitize_name(name: &str) -> String {
        name.trim().to_lowercase().replace([' ', '-'], "_")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_java_home() {
        assert!(!ValidationUtils::validate_java_home("/nonexistent/path"));
        // 注意：这个测试需要根据实际环境调整
    }

    #[test]
    fn test_validate_environment_name() {
        assert!(ValidationUtils::validate_environment_name("valid_name").is_ok());
        assert!(ValidationUtils::validate_environment_name("invalid/name").is_err());
        assert!(ValidationUtils::validate_environment_name("").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(ValidationUtils::validate_url("https://example.com").is_ok());
        assert!(ValidationUtils::validate_url("http://localhost:8080").is_ok());
        assert!(ValidationUtils::validate_url("invalid-url").is_err());
    }

    #[test]
    fn test_validate_api_key() {
        assert!(ValidationUtils::validate_api_key("${ENV_VAR}").is_ok());
        assert!(ValidationUtils::validate_api_key("sk-1234567890abcdef").is_ok());
        assert!(ValidationUtils::validate_api_key("short").is_err());
        assert!(ValidationUtils::validate_api_key("").is_err());
    }

    #[test]
    fn test_validate_version() {
        assert!(ValidationUtils::validate_version("1.0.0").is_ok());
        assert!(ValidationUtils::validate_version("17").is_ok());
        assert!(ValidationUtils::validate_version("21.0.3").is_ok());
        assert!(ValidationUtils::validate_version("").is_err());
        assert!(ValidationUtils::validate_version("invalid..version").is_err());
    }
}
