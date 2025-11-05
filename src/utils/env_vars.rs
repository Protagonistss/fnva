use std::collections::HashMap;
use std::env;

/// 环境变量工具
pub struct EnvVarUtils;

impl EnvVarUtils {
    /// 获取环境变量，处理变量引用
    pub fn get_with_expansion(var_name: &str) -> Result<String, String> {
        let value = env::var(var_name).map_err(|_| format!("Environment variable '{}' not found", var_name))?;
        Ok(Self::expand_variables(&value))
    }

    /// 展开字符串中的环境变量引用 (${VAR_NAME})
    pub fn expand_variables(input: &str) -> String {
        let mut result = input.to_string();

        // 简单的正则表达式匹配 ${VAR_NAME} 格式
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_start = start + 2;
                let var_end = start + end;

                if var_end > var_start {
                    let var_name = &result[var_start..var_end];
                    if let Ok(var_value) = env::var(var_name) {
                        // 替换整个 ${VAR_NAME} 为变量值
                        result.replace_range(start..=var_end, &var_value);
                    } else {
                        // 如果变量不存在，移除整个引用
                        result.replace_range(start..=var_end, "");
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        result
    }

    /// 设置环境变量
    pub fn set(key: &str, value: &str) -> Result<(), String> {
        env::set_var(key, value);
        Ok(())
    }

    /// 移除环境变量
    pub fn remove(key: &str) -> Result<(), String> {
        env::remove_var(key);
        Ok(())
    }

    /// 检查环境变量是否存在
    pub fn exists(key: &str) -> bool {
        env::var(key).is_ok()
    }

    /// 获取所有以指定前缀开头的环境变量
    pub fn get_with_prefix(prefix: &str) -> HashMap<String, String> {
        let mut result = HashMap::new();

        for (key, value) in env::vars() {
            if key.starts_with(prefix) {
                result.insert(key, value);
            }
        }

        result
    }

    /// 批量设置环境变量
    pub fn set_batch(vars: &HashMap<String, String>) -> Result<(), String> {
        for (key, value) in vars {
            env::set_var(key, value);
        }
        Ok(())
    }

    /// 批量移除环境变量
    pub fn remove_batch(keys: &[&str]) -> Result<(), String> {
        for key in keys {
            env::remove_var(key);
        }
        Ok(())
    }

    /// 创建环境变量的快照
    pub fn snapshot() -> HashMap<String, String> {
        env::vars().collect()
    }

    /// 从快照恢复环境变量
    pub fn restore_snapshot(snapshot: &HashMap<String, String>) -> Result<(), String> {
        // 清除当前所有环境变量（除了系统关键的）
        for (key, _) in env::vars() {
            if !Self::is_system_variable(&key) {
                env::remove_var(&key);
            }
        }

        // 恢复快照中的环境变量
        for (key, value) in snapshot {
            env::set_var(key, value);
        }

        Ok(())
    }

    /// 检查是否是系统关键环境变量
    fn is_system_variable(key: &str) -> bool {
        let system_vars = [
            "PATH", "HOME", "USER", "USERNAME", "USERPROFILE", "TEMP", "TMP",
            "COMSPEC", "OS", "PROCESSOR_ARCHITECTURE", "NUMBER_OF_PROCESSORS",
            "COMPUTERNAME", "SystemRoot", "ProgramFiles", "ProgramFiles(x86)",
            "CommonProgramFiles", "CommonProgramFiles(x86)", "ProgramData",
            "LOCALAPPDATA", "APPDATA", "HOMEDRIVE", "HOMEPATH"
        ];

        system_vars.iter().any(|&var| var.eq_ignore_ascii_case(key))
    }

    /// 获取 PATH 变量的所有路径
    pub fn get_paths() -> Vec<String> {
        let path_separator = if cfg!(target_os = "windows") { ';' } else { ':' };

        env::var("PATH")
            .unwrap_or_default()
            .split(path_separator)
            .map(|s| s.to_string())
            .collect()
    }

    /// 添加路径到 PATH
    pub fn add_to_path(path: &str, position: PathPosition) -> Result<(), String> {
        let path_separator = if cfg!(target_os = "windows") { ';' } else { ':' };

        let mut paths = Self::get_paths();

        // 避免重复添加
        if paths.iter().any(|p| p == path) {
            return Ok(());
        }

        match position {
            PathPosition::Front => {
                paths.insert(0, path.to_string());
            }
            PathPosition::Back => {
                paths.push(path.to_string());
            }
        }

        let new_path = paths.join(&path_separator.to_string());
        env::set_var("PATH", new_path);

        Ok(())
    }

    /// 从 PATH 移除路径
    pub fn remove_from_path(path: &str) -> Result<(), String> {
        let path_separator = if cfg!(target_os = "windows") { ';' } else { ':' };

        let paths: Vec<String> = Self::get_paths()
            .into_iter()
            .filter(|p| p != path)
            .collect();

        let new_path = paths.join(&path_separator.to_string());
        env::set_var("PATH", new_path);

        Ok(())
    }

    /// 清理 PATH 中的重复项
    pub fn clean_path() -> Result<(), String> {
        let path_separator = if cfg!(target_os = "windows") { ';' } else { ':' };

        let mut seen = std::collections::HashSet::new();
        let cleaned_paths: Vec<String> = Self::get_paths()
            .into_iter()
            .filter(|p| {
                if seen.contains(p) {
                    false
                } else {
                    seen.insert(p.clone());
                    true
                }
            })
            .collect();

        let new_path = cleaned_paths.join(&path_separator.to_string());
        env::set_var("PATH", new_path);

        Ok(())
    }

    /// 验证环境变量名称
    pub fn validate_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Environment variable name cannot be empty".to_string());
        }

        // 环境变量名称只能包含字母、数字和下划线
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("Environment variable name can only contain letters, numbers and underscores".to_string());
        }

        // 不能以数字开头
        if name.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return Err("Environment variable name cannot start with a number".to_string());
        }

        Ok(())
    }

    /// 获取环境变量的详细信息
    pub fn get_info(key: &str) -> Result<EnvVarInfo, String> {
        let value = env::var(key).map_err(|_| format!("Environment variable '{}' not found", key))?;

        Ok(EnvVarInfo {
            name: key.to_string(),
            value: value.clone(),
            is_system: Self::is_system_variable(key),
            has_expansion: value.contains("${"),
            expanded_value: Self::expand_variables(&value),
        })
    }

    /// 导出环境变量到字符串
    pub fn export_vars(vars: &HashMap<String, String>, shell_type: ShellType) -> String {
        let mut result = String::new();

        for (key, value) in vars {
            match shell_type {
                ShellType::PowerShell => {
                    result.push_str(&format!("$env:{} = \"{}\"\n", key, value));
                }
                ShellType::Bash | ShellType::Zsh => {
                    result.push_str(&format!("export {}=\"{}\"\n", key, value));
                }
                ShellType::Fish => {
                    result.push_str(&format!("set -gx {} \"{}\"\n", key, value));
                }
                ShellType::Cmd => {
                    result.push_str(&format!("set {}={}\n", key, value));
                }
                ShellType::Unknown => {
                    result.push_str(&format!("{}={}\n", key, value));
                }
            }
        }

        result
    }
}

/// PATH 操作位置
pub enum PathPosition {
    Front,
    Back,
}

/// Shell 类型（简化版，避免循环依赖）
#[derive(Debug, Clone, Copy)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Cmd,
    Unknown,
}

/// 环境变量信息
#[derive(Debug)]
pub struct EnvVarInfo {
    /// 变量名称
    pub name: String,
    /// 原始值
    pub value: String,
    /// 是否是系统变量
    pub is_system: bool,
    /// 是否包含变量引用
    pub has_expansion: bool,
    /// 展开后的值
    pub expanded_value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_name() {
        assert!(EnvVarUtils::validate_name("VALID_VAR").is_ok());
        assert!(EnvVarUtils::validate_name("VALID_VAR_123").is_ok());
        assert!(EnvVarUtils::validate_name("123INVALID").is_err());
        assert!(EnvVarUtils::validate_name("INVALID-VAR").is_err());
        assert!(EnvVarUtils::validate_name("").is_err());
    }

    #[test]
    fn test_expand_variables() {
        env::set_var("TEST_VAR", "test_value");
        assert_eq!(EnvVarUtils::expand_variables("prefix_${TEST_VAR}_suffix"), "prefix_test_value_suffix");
        assert_eq!(EnvVarUtils::expand_variables("no_variables"), "no_variables");

        // 测试不存在的变量
        assert_eq!(EnvVarUtils::expand_variables("${NON_EXISTENT}"), "");
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_path_operations() {
        let original_path = EnvVarUtils::get_paths();

        // 添加路径
        let test_path = "/test/path";
        EnvVarUtils::add_to_path(test_path, PathPosition::Front).unwrap();
        assert!(EnvVarUtils::get_paths().iter().any(|p| p == test_path));

        // 清理重复项
        EnvVarUtils::add_to_path(test_path, PathPosition::Back).unwrap();
        let paths_before_clean = EnvVarUtils::get_paths();
        EnvVarUtils::clean_path().unwrap();
        let paths_after_clean = EnvVarUtils::get_paths();
        assert_eq!(paths_before_clean.len(), paths_after_clean.len());

        // 移除路径
        EnvVarUtils::remove_from_path(test_path).unwrap();
        assert!(!EnvVarUtils::get_paths().iter().any(|p| p == test_path));

        // 恢复原始 PATH
        env::set_var("PATH", original_path.join(if cfg!(target_os = "windows") { ";" } else { ":" }));
    }
}