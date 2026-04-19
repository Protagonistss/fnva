use crate::config::Config;
use crate::infrastructure::shell::platform::ShellType;
use std::path::PathBuf;

/// Shell Hook 管理器
pub struct ShellHook;

impl ShellHook {
    /// 获取当前环境状态文件路径
    fn get_current_env_file() -> Result<PathBuf, String> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| "Cannot get user home directory".to_string())?;
        Ok(home_dir.join(".fnva").join("current_env"))
    }

    /// 读取当前激活的环境
    pub fn get_current_environment() -> Result<Option<String>, String> {
        let current_env_file = Self::get_current_env_file()?;

        if !current_env_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&current_env_file)
            .map_err(|e| format!("Failed to read current environment file: {e}"))?;

        let env_name = content.trim().to_string();
        if env_name.is_empty() {
            Ok(None)
        } else {
            Ok(Some(env_name))
        }
    }

    /// 设置当前激活的环境
    pub fn set_current_environment(env_name: &str) -> Result<(), String> {
        let current_env_file = Self::get_current_env_file()?;

        // 确保目录存在
        if let Some(parent) = current_env_file.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create fnva directory: {e}"))?;
        }

        std::fs::write(&current_env_file, env_name)
            .map_err(|e| format!("Failed to write current environment file: {e}"))?;

        Ok(())
    }

    /// 清除当前环境
    pub fn clear_current_environment() -> Result<(), String> {
        let current_env_file = Self::get_current_env_file()?;

        if current_env_file.exists() {
            std::fs::remove_file(&current_env_file)
                .map_err(|e| format!("Failed to remove current environment file: {e}"))?;
        }

        Ok(())
    }

    /// 应用环境变量设置（改进版本）
    pub fn apply_environment(env_name: &str) -> Result<(), String> {
        // 带重试机制的配置加载
        let config = Config::load()?;

        let env = config
            .get_java_env(env_name)
            .ok_or_else(|| format!("Java environment '{env_name}' not found"))?;

        // 验证路径（更详细的验证）
        if !std::path::Path::new(&env.java_home).exists() {
            return Err(format!("JAVA_HOME path does not exist: {}", env.java_home));
        }

        if !crate::utils::validate_java_home(&env.java_home) {
            return Err(format!("Invalid JAVA_HOME path: {}", env.java_home));
        }

        // 验证 java.exe 是否存在
        let java_exe = if cfg!(target_os = "windows") {
            format!("{}\\bin\\java.exe", env.java_home)
        } else {
            format!("{}/bin/java", env.java_home)
        };

        if !std::path::Path::new(&java_exe).exists() {
            return Err(format!("Java executable not found: {java_exe}"));
        }

        // 清理 PATH 中的现有 Java 路径
        let cleaned_path = Self::clean_java_paths(&env.java_home)?;

        // 设置环境变量
        std::env::set_var("JAVA_HOME", &env.java_home);
        std::env::set_var("PATH", &cleaned_path);

        // 验证设置是否成功
        if let Ok(current_java_home) = std::env::var("JAVA_HOME") {
            if current_java_home != env.java_home {
                return Err("Failed to set JAVA_HOME environment variable".to_string());
            }
        } else {
            return Err("JAVA_HOME environment variable not set".to_string());
        }

        Ok(())
    }

    /// 清理 PATH 中的现有 Java 路径，并添加新的 Java 路径
    fn clean_java_paths(new_java_home: &str) -> Result<String, String> {
        let bin_path = if cfg!(target_os = "windows") {
            format!("{new_java_home}\\bin")
        } else {
            format!("{new_java_home}/bin")
        };

        let current_path = std::env::var("PATH").unwrap_or_default();
        let path_separator = if cfg!(target_os = "windows") {
            ';'
        } else {
            ':'
        };

        let path_parts: Vec<String> = current_path
            .split(path_separator)
            .filter_map(|part| {
                let trimmed = part.trim();
                // 过滤掉 Java 相关的路径
                if trimmed.to_lowercase().contains("java")
                    || trimmed.to_lowercase().contains("jdk")
                    || trimmed.contains(new_java_home)
                {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            })
            .collect();

        // 将新的 Java 路径放在最前面
        let mut new_path_parts = vec![bin_path];
        new_path_parts.extend(path_parts);

        Ok(new_path_parts.join(&path_separator.to_string()))
    }

    /// 安全的环境切换，包含验证和回滚机制
    pub fn safe_apply_environment(env_name: &str) -> Result<String, String> {
        // 备份当前环境
        let old_java_home = std::env::var("JAVA_HOME").ok();
        let old_path = std::env::var("PATH").ok();

        match Self::apply_environment(env_name) {
            Ok(()) => {
                // 验证 Java 是否工作正常
                let java_version = Self::test_java_version();
                match java_version {
                    Ok(version) => Ok(format!("Successfully switched to Java environment: {}\nJAVA_HOME: {}\nJava version: {}",
                        env_name,
                        std::env::var("JAVA_HOME").unwrap_or_default(),
                        version)),
                    Err(e) => {
                        // 回滚环境变量
                        if let Some(old_home) = old_java_home {
                            std::env::set_var("JAVA_HOME", old_home);
                        } else {
                            std::env::remove_var("JAVA_HOME");
                        }
                        if let Some(old_path_val) = old_path {
                            std::env::set_var("PATH", old_path_val);
                        }
                        Err(format!("Environment switch failed, rolled back: {e}"))
                    }
                }
            }
            Err(e) => Err(e),
        }
    }

    /// 测试 Java 版本以验证环境切换是否成功
    fn test_java_version() -> Result<String, String> {
        let java_exe = if cfg!(target_os = "windows") {
            "java.exe"
        } else {
            "java"
        };

        use std::process::Command;
        let output = Command::new(java_exe)
            .arg("-version")
            .output()
            .map_err(|e| format!("Failed to execute java -version: {e}"))?;

        if output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let lines: Vec<&str> = stderr.lines().collect();
            if let Some(version_line) = lines.first() {
                Ok(version_line.to_string())
            } else {
                Ok("Java version detected".to_string())
            }
        } else {
            Err(format!(
                "Java -version command failed with exit code: {}",
                output.status
            ))
        }
    }

    /// 检查并应用当前环境（如果存在）
    pub fn check_and_apply_current() -> Result<(), String> {
        if let Some(current_env) = Self::get_current_environment()? {
            Self::apply_environment(&current_env)?;
        }
        Ok(())
    }

    /// --use-on-cd �ű���ѡ���� shell
    pub fn generate_use_on_cd_script(shell: ShellType) -> Result<String, String> {
        match shell {
            ShellType::PowerShell => Self::generate_powershell_hook(),
            ShellType::Cmd | ShellType::Bash | ShellType::Zsh | ShellType::Fish => {
                Err("Current shell is not supported for --use-on-cd yet. Please run 'fnva java install-hook' instead.".to_string())
            }
            ShellType::Unknown => {
                Err("Cannot detect shell type. Use --shell to specify it explicitly.".to_string())
            }
        }
    }

    /// 生成 PowerShell Hook 脚本
    pub fn generate_powershell_hook() -> Result<String, String> {
        let script = r#"# fnva PowerShell Hook - Auto environment switching
# Add this to your PowerShell Profile with: Invoke-Expression (& fnva env --use-on-cd | Out-String)

# Store original prompt function if it exists
if (Get-Command prompt -ErrorAction SilentlyContinue) {
    $originalPrompt = Get-Content function:prompt
} else {
    $originalPrompt = { "PS $($executionContext.SessionState.Path.CurrentLocation)$('>' * ($nestedPromptLevel + 1)) " }
}

# Enhanced prompt function with fnva hook
function prompt {
    # Apply fnva environment from current_env file
    $envFile = "$env:USERPROFILE\.fnva\current_env"
    if (Test-Path $envFile) {
        try {
            $currentEnv = Get-Content $envFile -Raw -ErrorAction SilentlyContinue
            $currentEnv = $currentEnv.Trim()

            if ($currentEnv -and $env:FNVA_CURRENT_ENV -ne $currentEnv) {
                # Use fnva command to get environment details in JSON format
                $fnvaOutput = & fnva java current --json 2>$null
                if ($fnvaOutput) {
                    try {
                        $envData = $fnvaOutput | ConvertFrom-Json
                        if ($envData.name -and $envData.java_home) {
                            # Clean existing Java paths from PATH
                            $pathParts = $env:PATH -split ';'
                            $cleanPath = @()
                            foreach ($part in $pathParts) {
                                if ($part -notmatch 'java' -and $part -notmatch 'jdk') {
                                    $cleanPath += $part
                                }
                            }

                            # Set new environment
                            $env:JAVA_HOME = $envData.java_home
                            $binPath = Join-Path $envData.java_home "bin"
                            $env:PATH = "$binPath;" + ($cleanPath -join ';')
                            $env:FNVA_CURRENT_ENV = $envData.name
                        }
                    } catch {
                        # Fallback to simple method if JSON parsing fails
                        & fnva java use $currentEnv 2>$null
                        $env:FNVA_CURRENT_ENV = $currentEnv
                    }
                }
            }
        } catch {
            # Silently continue on error
        }
    }

    # Call original prompt
    & $originalPrompt
}

# Initialize FNVA_CURRENT_ENV to avoid initial switch
if (-not $env:FNVA_CURRENT_ENV) {
    $env:FNVA_CURRENT_ENV = ""
}

Write-Host "fnva PowerShell Hook installed" -ForegroundColor Green
Write-Host "Auto Java environment switching enabled" -ForegroundColor Cyan"#;

        Ok(script.to_string())
    }

    /// 生成 CMD Hook 脚本
    pub fn generate_cmd_hook() -> Result<String, String> {
        let script = r#"@echo off
REM fnva CMD Hook
REM Create a wrapper for cmd.exe

setlocal enabledelayedexpansion
set "fnvaDir=%USERPROFILE%\.fnva"
set "hookScript=%fnvaDir%\fnva-cmd-hook.bat"

REM Create the hook script
(
echo @echo off
echo REM fnva CMD Hook - Applied before each command
echo setlocal enabledelayedexpansion
echo.
echo REM Check for current environment
echo if exist "%fnvaDir%\current_env" ^(
echo     set /p currentEnv=^<"%fnvaDir%\current_env"
echo     if "!currentEnv!" neq "" ^(
echo         REM Apply environment variables
echo         if exist "%fnvaDir%\config.toml" ^(
echo             REM Simple parsing for java_home
echo             for /f "tokens=2 delims==" %%a in ^('findstr /C:"current_java_env = "!currentEnv!"" "%fnvaDir%\config.toml"'^) do ^(
echo                 REM Found current environment, now find java_home
echo                 set "foundEnv="
echo                 for /f "usebackq tokens=*" %%b in ^("%fnvaDir%\config.toml"^) do ^(
echo                     set "line=%%b"
echo                     if "!line!"=="[[java_environments]]" set "foundEnv=1"
echo                     if "!foundEnv!"=="1" ^(
echo                         if "!line!"=="name = "!currentEnv!"" ^(
echo                             REM Found the environment, look for java_home
echo                             set "lookingForHome=1"
echo                         ^) else if "!lookingForHome!"=="1" ^(
echo                             if "!line!"=="java_home = " ^(
echo                                 set "javaHome=!line:java_home = =!"
echo                                 set "JAVA_HOME=!javaHome!"
echo                                 set "PATH=!javaHome!\bin;!PATH!"
echo                                 goto :done
echo                             ^)
echo                         ^)
echo                     ^)
echo                 ^)
echo             ^)
echo         ^)
echo     ^)
echo ^)
echo :done
echo.
) > "%hookScript%"

REM Create a fnva wrapper command
echo @echo off > "%fnvaDir%\fnva.bat"
echo call "%hookScript%" >> "%fnvaDir%\fnva.bat"
echo cargo run --manifest-path "%~dp0\Cargo.toml" %%* >> "%fnvaDir%\fnva.bat"

echo 🚀 fnva CMD Hook installed
echo 💡 fnva java use jdk21 will now work in current shell
echo 📋 Add %fnvaDir% to your PATH for easy access"#;

        Ok(script.to_string())
    }

    /// 生成 Hook 安装脚本
    pub fn generate_hook_installation() -> Result<String, String> {
        let ps_hook = Self::generate_powershell_hook()?;
        let cmd_hook = Self::generate_cmd_hook()?;

        Ok(format!(
            "✅ Shell Hook installation scripts generated\n\
            \n\
            📋 PowerShell Hook:\n\
            {ps_hook}\n\
            \n\
            📋 CMD Hook:\n\
            {cmd_hook}\n\
            \n\
            🚀 Installation Instructions:\n\
            \n\
            PowerShell:\n\
            1. Run: notepad $PROFILE\n\
            2. Copy the PowerShell script above into the file\n\
            3. Restart PowerShell\n\
            \n\
            CMD:\n\
            1. Run the CMD script above\n\
            2. Add fnva directory to PATH\n\
            3. Restart CMD\n\
            \n\
            📖 After installation:\n\
            fnva java use jdk21  # Will work immediately in current shell!"
        ))
    }

    /// 生成 Hook 卸载脚本
    pub fn generate_hook_uninstallation() -> Result<String, String> {
        let script = r#"# fnva Hook Uninstallation

## PowerShell
Remove the fnva hook from your PowerShell profile:
1. Run: notepad $PROFILE
2. Delete the fnva hook section
3. Restart PowerShell

## CMD
1. Remove fnva directory from PATH
2. Delete %USERPROFILE%\.fnva\fnva-cmd-hook.bat
3. Delete %USERPROFILE%\.fnva\fnva.bat

## Manual Cleanup
Remove the following files:
- %USERPROFILE%\.fnva\fnva-cmd-hook.bat
- %USERPROFILE%\.fnva\fnva.bat
- fnva hook section from PowerShell profile"#;

        Ok(script.to_string())
    }
}
