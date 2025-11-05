use std::env;

/// 操作系统类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsType {
    Windows,
    MacOS,
    Linux,
}

/// Shell 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Cmd,
    Unknown,
}

/// 获取当前操作系统类型
pub fn get_os_type() -> OsType {
    match env::consts::OS {
        "windows" => OsType::Windows,
        "macos" => OsType::MacOS,
        "linux" => OsType::Linux,
        _ => OsType::Linux, // 默认当作 Linux 处理
    }
}

/// 检测当前使用的 shell
pub fn detect_shell() -> ShellType {
    // Windows 平台优先检测 Windows shell
    if cfg!(target_os = "windows") {
        // Windows PowerShell 检测
        if let Ok(ps_module_path) = env::var("PSModulePath") {
            if !ps_module_path.is_empty() {
                // 检查是否在 PowerShell 中
                if let Ok(pwsh) = env::var("POWERSHELL_PROCESS") {
                    if !pwsh.is_empty() {
                        return ShellType::PowerShell;
                    }
                }
                // 另一种检测方式：检查 TERM_PROGRAM
                if env::var("TERM_PROGRAM").is_ok() {
                    // 可能是 PowerShell，但需要进一步确认
                    return ShellType::PowerShell;
                }
            }
        }

        // Windows CMD 检测
        if env::var("COMSPEC").is_ok() {
            // 检查是否在 CMD 中（通常 PowerShell 会有额外的环境变量）
            if env::var("PSModulePath").is_err() {
                return ShellType::Cmd;
            }
            // 如果 PSModulePath 存在，但 SHELL 变量也表明是 Unix shell，
            // 优先认为是 PowerShell（因为在 Windows 上运行）
            return ShellType::PowerShell;
        }
    }

    // 从环境变量检测 Unix shell
    if let Ok(shell) = env::var("SHELL") {
        if shell.contains("fish") {
            return ShellType::Fish;
        } else if shell.contains("zsh") {
            return ShellType::Zsh;
        } else if shell.contains("bash") {
            return ShellType::Bash;
        }
    }

    // 默认检测：根据操作系统
    match get_os_type() {
        OsType::Windows => {
            // Windows 默认尝试 PowerShell
            ShellType::PowerShell
        }
        OsType::MacOS | OsType::Linux => {
            // Unix-like 系统默认使用 bash
            ShellType::Bash
        }
    }
}

/// 生成设置环境变量的命令
pub fn generate_env_command(key: &str, value: &str, shell: ShellType) -> String {
    match shell {
        ShellType::Bash | ShellType::Zsh => {
            format!("export {}='{}'", key, escape_shell_value(value))
        }
        ShellType::Fish => {
            format!("set -gx {} '{}'", key, escape_shell_value(value))
        }
        ShellType::PowerShell => {
            format!("$env:{} = '{}'", key, escape_powershell_value(value))
        }
        ShellType::Cmd => {
            format!("set {}={}", key, escape_cmd_value(value))
        }
        ShellType::Unknown => {
            // 默认使用 bash 格式
            format!("export {}='{}'", key, escape_shell_value(value))
        }
    }
}

/// 生成设置 PATH 的命令（追加到 PATH）
pub fn generate_path_command(path_to_add: &str, shell: ShellType) -> String {
    match shell {
        ShellType::Bash | ShellType::Zsh => {
            format!("export PATH=\"{}:$PATH\"", escape_shell_value(path_to_add))
        }
        ShellType::Fish => {
            format!(
                "set -gx PATH \"{} $PATH\"",
                escape_shell_value(path_to_add)
            )
        }
        ShellType::PowerShell => {
            format!(
                "$env:PATH = '{};' + $env:PATH",
                escape_powershell_value(path_to_add)
            )
        }
        ShellType::Cmd => {
            format!("set PATH={};%PATH%", escape_cmd_value(path_to_add))
        }
        ShellType::Unknown => {
            format!("export PATH=\"{}:$PATH\"", escape_shell_value(path_to_add))
        }
    }
}

/// 转义 shell 值（bash/zsh）
fn escape_shell_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\'', "'\\''")
        .replace('$', "\\$")
        .replace('"', "\\\"")
        .replace('`', "\\`")
}

/// 转义 PowerShell 值
fn escape_powershell_value(value: &str) -> String {
    value.replace('\'', "''")
}

/// 转义 CMD 值
fn escape_cmd_value(value: &str) -> String {
    value.replace('&', "^&").replace('|', "^|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os_type() {
        let os = get_os_type();
        assert!(matches!(os, OsType::Windows | OsType::MacOS | OsType::Linux));
    }

    #[test]
    fn test_generate_env_command() {
        let cmd = generate_env_command("JAVA_HOME", "/usr/lib/jvm/java-17", ShellType::Bash);
        assert!(cmd.contains("export"));
        assert!(cmd.contains("JAVA_HOME"));
    }

    #[test]
    fn test_escape_shell_value() {
        let escaped = escape_shell_value("path/with'spaces");
        assert!(!escaped.contains('\''));
    }
}
