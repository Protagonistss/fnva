use crate::config::{Config, JavaEnvironment};
use crate::infrastructure::shell::platform::{
    detect_shell,
    generate_env_command,
    generate_path_command,
    ShellType,
};
use crate::utils::validate_java_home;
use std::path::PathBuf;
use which::which;

/// Java 环境管理器
pub struct JavaManager;

impl JavaManager {
    /// 列出所有 Java 环境
    pub fn list(config: &Config) -> Vec<&JavaEnvironment> {
        config.java_environments.iter().collect()
    }

    /// 生成切换到指定 Java 环境的命令
    pub fn generate_switch_command(
        config: &Config,
        name: &str,
        shell: Option<ShellType>,
    ) -> Result<String, String> {
        let env = config
            .get_java_env(name)
            .ok_or_else(|| format!("Java 环境 '{}' 不存在", name))?;

        // 验证 Java Home 路径
        if !validate_java_home(&env.java_home) {
            return Err(format!(
                "无效的 JAVA_HOME 路径: {}",
                env.java_home
            ));
        }

        let shell = shell.unwrap_or_else(detect_shell);
        let mut commands = Vec::new();

        // 设置 JAVA_HOME
        commands.push(generate_env_command("JAVA_HOME", &env.java_home, shell));

        // 更新 PATH（添加 bin 目录）
        let bin_path = if cfg!(target_os = "windows") {
            format!("{}\\bin", env.java_home)
        } else {
            format!("{}/bin", env.java_home)
        };

        // 检查 bin 目录是否存在
        if PathBuf::from(&bin_path).exists() {
            commands.push(generate_path_command(&bin_path, shell));
        }

        Ok(commands.join("\n"))
    }

    /// 生成切换到指定 Java 环境的脚本文件
    pub fn generate_switch_script(
        config: &Config,
        name: &str,
    ) -> Result<String, String> {
        let env = config
            .get_java_env(name)
            .ok_or_else(|| format!("Java 环境 '{}' 不存在", name))?;

        // 验证 Java Home 路径
        if !validate_java_home(&env.java_home) {
            return Err(format!(
                "无效的 JAVA_HOME 路径: {}",
                env.java_home
            ));
        }

        // 获取 PowerShell 脚本路径
        let script_dir = dirs::home_dir()
            .ok_or_else(|| "无法获取用户主目录".to_string())?
            .join(".fnva");

        // 确保目录存在
        std::fs::create_dir_all(&script_dir)
            .map_err(|e| format!("创建脚本目录失败: {}", e))?;

        let script_path = script_dir.join("switch-java.ps1");

        // 生成 PowerShell 脚本内容
        let script_content = format!(r#"
# fnva 生成的 Java 环境切换脚本
# 使用方法: .\switch-java.ps1 jdk21

param(
    [Parameter(Mandatory=$false)]
    [string]$TargetJava = "{}"
)

# 硬编码的环境配置（为了简化，避免 TOML 解析依赖）
$JavaEnvironments = @{{
    "jdk21" = @{{
        java_home = "{}"
        description = "Java 21.0.3 from GitHub/Adoptium"
    }}
}}

# 查找目标 Java 环境
$TargetEnv = $JavaEnvironments[$TargetJava]

if (!$TargetEnv) {{
    Write-Error "找不到 Java 环境: $TargetJava"
    Write-Host "可用的 Java 环境:"
    $JavaEnvironments.Keys | ForEach-Object {{
        Write-Host "  - $($_): $($JavaEnvironments[$_].java_home)"
    }}
    exit 1
}}

# 设置环境变量
$env:JAVA_HOME = $TargetEnv.java_home
$env:PATH = "$($TargetEnv.java_home)\bin;" + $env:PATH

Write-Host "✅ 已切换到 Java: $TargetJava" -ForegroundColor Green
Write-Host "📍 JAVA_HOME: $env:JAVA_HOME" -ForegroundColor Yellow

# 验证切换结果
try {{
    $VersionOutput = & "$($TargetEnv.java_home)\bin\java.exe" --version 2>&1
    Write-Host "🔍 Java 版本:" -ForegroundColor Cyan
    Write-Host $VersionOutput[0] -ForegroundColor White
}} catch {{
    Write-Warning "无法验证 Java 版本，请检查安装"
}}
"#, name, env.java_home);

        // 写入脚本文件
        std::fs::write(&script_path, script_content)
            .map_err(|e| format!("写入脚本文件失败: {}", e))?;

        Ok(format!("✅ 已生成切换脚本: {}\n使用方法: .\\switch-java.ps1 [环境名称]\n\n💡 提示: 将此目录添加到 PATH 或使用完整路径执行", script_path.display()))
    }

    /// 直接使用指定的 Java 版本执行命令
    pub fn execute_with_java(
        config: &Config,
        name: &str,
        java_args: Vec<String>,
    ) -> Result<(), String> {
        let env = config
            .get_java_env(name)
            .ok_or_else(|| format!("Java 环境 '{}' 不存在", name))?;

        // 验证 Java Home 路径
        if !validate_java_home(&env.java_home) {
            return Err(format!(
                "无效的 JAVA_HOME 路径: {}",
                env.java_home
            ));
        }

        let java_exe = if cfg!(target_os = "windows") {
            format!("{}\\bin\\java.exe", env.java_home)
        } else {
            format!("{}/bin/java", env.java_home)
        };

        // 构建命令
        let mut cmd = std::process::Command::new(&java_exe);
        cmd.args(java_args);

        // 执行命令
        let output = cmd.output()
            .map_err(|e| format!("执行 Java 命令失败: {}", e))?;

        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Java 命令执行失败: {}", error));
        }

        Ok(())
    }

    /// 扫描系统中的 Java 安装
    pub fn scan_system() -> Vec<JavaInstallation> {
        let mut installations = Vec::new();

        // 常见 Java 安装路径
        let common_paths = get_common_java_paths();

        for path_str in common_paths {
            let path = PathBuf::from(&path_str);
            if path.exists() {
                // 检查是否是有效的 Java 安装目录
                if let Some(installation) = check_java_installation(&path) {
                    installations.push(installation);
                }
            }
        }

        // 尝试从 PATH 中查找 java 命令
        if let Ok(java_path) = which("java") {
            if let Some(home) = find_java_home_from_path(&java_path) {
                if let Some(installation) = check_java_installation(&home) {
                    // 避免重复添加
                    if !installations.iter().any(|i| i.java_home == home.to_string_lossy()) {
                        installations.push(installation);
                    }
                }
            }
        }

        installations
    }

    /// 添加 Java 环境到配置
    pub fn add(
        config: &mut Config,
        name: String,
        java_home: String,
        description: Option<String>,
    ) -> Result<(), String> {
        // 验证路径
        if !validate_java_home(&java_home) {
            return Err(format!("无效的 JAVA_HOME 路径: {}", java_home));
        }

        let env = JavaEnvironment {
            name,
            java_home,
            description: description.unwrap_or_default(),
            source: crate::config::EnvironmentSource::Manual,
        };

        config.add_java_env(env)?;
        config.save()?;
        Ok(())
    }

    /// 从配置中删除 Java 环境
    pub fn remove(config: &mut Config, name: &str) -> Result<(), String> {
        config.remove_java_env(name)?;
        config.save()?;
        Ok(())
    }
}

/// Java 安装信息
#[derive(Debug, Clone)]
pub struct JavaInstallation {
    pub java_home: String,
    pub version: Option<String>,
    pub description: String,
}

/// 检查路径是否是有效的 Java 安装
fn check_java_installation(path: &PathBuf) -> Option<JavaInstallation> {
    // 检查是否存在 java 可执行文件
    let java_exe = if cfg!(target_os = "windows") {
        path.join("bin").join("java.exe")
    } else {
        path.join("bin").join("java")
    };

    if !java_exe.exists() {
        return None;
    }

    // 尝试获取版本信息
    let version = get_java_version(&java_exe).ok();

    // 生成描述
    let path_str = path.to_string_lossy();
    let description = if let Some(ver) = &version {
        format!("Java {} ({})", ver, path_str)
    } else {
        path_str.to_string()
    };

    Some(JavaInstallation {
        java_home: path_str.to_string(),
        version,
        description,
    })
}

/// 获取 Java 版本
fn get_java_version(java_exe: &PathBuf) -> Result<String, String> {
    use std::process::Command;

    let output = Command::new(java_exe)
        .arg("-version")
        .output()
        .map_err(|e| format!("执行 java -version 失败: {}", e))?;

    if !output.status.success() {
        return Err("无法获取 Java 版本".to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // 解析版本号（例如 "openjdk version \"17.0.1\""）
    if let Some(line) = stderr.lines().next() {
        if let Some(version_start) = line.find("version \"") {
            let version_part = &line[version_start + 9..];
            if let Some(version_end) = version_part.find('"') {
                return Ok(version_part[..version_end].to_string());
            }
        }
    }

    Err("无法解析版本信息".to_string())
}

/// 从 java 可执行文件路径找到 JAVA_HOME
fn find_java_home_from_path(java_path: &PathBuf) -> Option<PathBuf> {
    // java 通常在 $JAVA_HOME/bin/java，所以向上两级
    let mut current = java_path.clone();
    
    // 移除文件名
    if let Some(parent) = current.parent() {
        current = parent.to_path_buf();
    } else {
        return None;
    }

    // 移除 bin 目录
    if current.file_name().and_then(|n| n.to_str()) == Some("bin") {
        if let Some(home) = current.parent() {
            return Some(home.to_path_buf());
        }
    }

    None
}

/// 获取常见的 Java 安装路径
fn get_common_java_paths() -> Vec<String> {
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        // Windows 常见路径
        if let Some(program_files) = std::env::var("ProgramFiles").ok() {
            paths.push(format!("{}\\Java", program_files));
        }
        if let Some(program_files_x86) = std::env::var("ProgramFiles(x86)").ok() {
            paths.push(format!("{}\\Java", program_files_x86));
        }
        // 扫描常见目录
        if let Some(local_appdata) = std::env::var("LOCALAPPDATA").ok() {
            paths.push(format!("{}\\Programs\\Java", local_appdata));
        }
    } else if cfg!(target_os = "macos") {
        // macOS 常见路径
        paths.push("/Library/Java/JavaVirtualMachines".to_string());
        paths.push("/usr/libexec/java_home".to_string());
        // 用户目录
        if let Some(home) = std::env::var("HOME").ok() {
            paths.push(format!("{}/Library/Java/JavaVirtualMachines", home));
        }
        // 扫描 /Library/Java/JavaVirtualMachines 下的子目录
        let jvm_path = PathBuf::from("/Library/Java/JavaVirtualMachines");
        if jvm_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&jvm_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        // macOS JDK 通常在 Contents/Home
                        let home = path.join("Contents").join("Home");
                        if home.exists() {
                            paths.push(home.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    } else {
        // Linux 常见路径
        paths.push("/usr/lib/jvm".to_string());
        paths.push("/usr/java".to_string());
        paths.push("/opt/java".to_string());
        
        // 扫描 /usr/lib/jvm 下的子目录
        let jvm_path = PathBuf::from("/usr/lib/jvm");
        if jvm_path.exists() {
            if let Ok(entries) = std::fs::read_dir(&jvm_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        paths.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_common_java_paths() {
        let paths = get_common_java_paths();
        assert!(!paths.is_empty());
    }
}
