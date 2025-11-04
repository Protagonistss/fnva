use std::path::PathBuf;

/// 验证 JAVA_HOME 路径是否有效
pub fn validate_java_home(java_home: &str) -> bool {
    let path = PathBuf::from(java_home);
    
    if !path.exists() {
        return false;
    }

    if !path.is_dir() {
        return false;
    }

    // 检查是否存在 bin 目录
    let bin_path = path.join("bin");
    if !bin_path.exists() {
        return false;
    }

    // 检查是否存在 java 可执行文件
    let java_exe = if cfg!(target_os = "windows") {
        bin_path.join("java.exe")
    } else {
        bin_path.join("java")
    };

    java_exe.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_java_home_invalid_path() {
        assert!(!validate_java_home("/nonexistent/path"));
    }
}
