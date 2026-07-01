use crate::config::Config;
use std::path::Path;

/// Shell 集成管理器
pub struct ShellIntegration;

impl ShellIntegration {
    /// 生成环境切换脚本
    pub fn generate_env_script(config: &Config, env_name: &str) -> Result<String, String> {
        let env = config
            .get_java_env(env_name)
            .ok_or_else(|| format!("Java environment '{env_name}' not found"))?;

        // 验证 Java Home 路径
        if !crate::utils::validate_java_home(&env.java_home) {
            return Err(format!("Invalid JAVA_HOME path: {}", env.java_home));
        }

        // 获取脚本目录
        let script_dir = dirs::home_dir()
            .ok_or_else(|| "Cannot get user home directory".to_string())?
            .join(".fnva");

        // 确保目录存在
        std::fs::create_dir_all(&script_dir)
            .map_err(|e| format!("Failed to create script directory: {e}"))?;

        let powershell_script = script_dir.join("fnva-env.ps1");
        let batch_script = script_dir.join("fnva-env.bat");

        // 生成 PowerShell 脚本
        let ps1_content = Self::generate_powershell_script(config, env_name)?;
        std::fs::write(&powershell_script, ps1_content)
            .map_err(|e| format!("Failed to write PowerShell script: {e}"))?;

        // 生成批处理脚本
        let bat_content = Self::generate_batch_script(config, env_name)?;
        std::fs::write(&batch_script, bat_content)
            .map_err(|e| format!("Failed to write batch script: {e}"))?;

        Ok(format!(
            "Environment switch script generated\n\
            PowerShell: {}\n\
            CMD: {}\n\
            \n\
            Usage:\n\
            PowerShell: . fnva-env.ps1 {}\n\
            CMD: fnva-env.bat {}\n\
            \n\
            Tip: install shell integration for a better experience:\n\
            fnva env",
            powershell_script.display(),
            batch_script.display(),
            env_name,
            env_name
        ))
    }

    /// 生成 PowerShell 脚本内容
    fn generate_powershell_script(config: &Config, _env_name: &str) -> Result<String, String> {
        let mut script_content = String::new();

        // 脚本头部注释
        script_content.push_str("# fnva environment switch script (PowerShell)\n");
        script_content.push_str("# Usage: . fnva-env.ps1 [environment_name]\n\n");

        // 参数处理
        script_content.push_str("param(\n");
        script_content.push_str("    [Parameter(Mandatory=$false)]\n");
        script_content.push_str("    [string]$EnvName = \"\"\n");
        script_content.push_str(")\n\n");

        // 环境变量定义（硬编码，避免 TOML 解析依赖）
        script_content.push_str("$JavaEnvironments = @{\n");
        for env in &config.java_environments {
            script_content.push_str(&format!(
                "    \"{}\" = @{{\n        java_home = \"{}\"\n        description = \"{}\"\n    }}\n",
                env.name,
                env.java_home.replace('\\', "\\\\"),
                env.description.replace('"', "\\\"")
            ));
        }
        script_content.push_str("}\n\n");

        // 当前激活环境（从配置读取）
        if let Some(current) = &config.current_java_env {
            script_content.push_str(&format!("$CurrentEnv = \"{current}\"\n\n"));
        }

        // 确定目标环境
        script_content
            .push_str("$TargetEnv = if ($EnvName -eq \"\") { $CurrentEnv } else { $EnvName }\n");
        script_content.push_str("if (!$TargetEnv) {\n");
        script_content
            .push_str("    Write-Error \"No environment specified and no current environment\"\n");
        script_content.push_str("    exit 1\n");
        script_content.push_str("}\n\n");

        // 查找环境配置
        script_content.push_str("$EnvConfig = $JavaEnvironments[$TargetEnv]\n");
        script_content.push_str("if (!$EnvConfig) {\n");
        script_content.push_str("    Write-Error \"Java environment not found: $TargetEnv\"\n");
        script_content.push_str("    Write-Host \"Available Java environments:\"\n");
        script_content.push_str("    $JavaEnvironments.Keys | ForEach-Object {\n");
        script_content
            .push_str("        Write-Host \"  - $($_): $($JavaEnvironments[$_].java_home)\"\n");
        script_content.push_str("    }\n");
        script_content.push_str("    exit 1\n");
        script_content.push_str("}\n\n");

        // 改进 PATH 管理，确保目标 Java 在最前面
        script_content.push_str("# Remove existing Java paths from PATH\n");
        script_content.push_str("$oldPath = $env:PATH\n");
        script_content.push_str("$pathParts = $oldPath -split ';'\n");
        script_content.push_str("$cleanPath = @()\n");
        script_content.push_str("foreach ($part in $pathParts) {\n");
        script_content.push_str("    if ($part -notmatch 'java' -and $part -notmatch 'jdk') {\n");
        script_content.push_str("        $cleanPath += $part\n");
        script_content.push_str("    }\n");
        script_content.push_str("}\n");

        // 设置环境变量
        script_content.push_str("$env:JAVA_HOME = $EnvConfig.java_home\n");
        script_content.push_str("$binPath = Join-Path $EnvConfig.java_home \"bin\"\n");
        script_content.push_str("$newPath = $binPath + \";\" + ($cleanPath -join \";\")\n");
        script_content.push_str("$env:PATH = $newPath\n\n");

        // 验证切换结果
        script_content.push_str("# Verify the switch\n");
        script_content.push_str("$javaExe = Join-Path $binPath \"java.exe\"\n");
        script_content.push_str("if (Test-Path $javaExe) {\n");
        script_content.push_str("    Write-Host \"Successfully switched to Java environment: $TargetEnv\" -ForegroundColor Green\n");
        script_content
            .push_str("    Write-Host \"JAVA_HOME: $env:JAVA_HOME\" -ForegroundColor Yellow\n");
        script_content.push_str("    try {\n");
        script_content.push_str("        $version = & $javaExe --version 2>&1\n");
        script_content.push_str(
            "        Write-Host \"Java version: $($version[0])\" -ForegroundColor Cyan\n",
        );
        script_content.push_str("    } catch {\n");
        script_content.push_str("        Write-Warning \"Cannot verify Java version\"\n");
        script_content.push_str("    }\n");
        script_content.push_str("} else {\n");
        script_content.push_str("    Write-Error \"Java executable not found at: $javaExe\"\n");
        script_content.push_str("    exit 1\n");
        script_content.push_str("}\n");

        Ok(script_content)
    }

    /// 生成批处理脚本内容
    fn generate_batch_script(config: &Config, env_name: &str) -> Result<String, String> {
        let mut script_content = String::new();

        // 脚本头部注释
        script_content.push_str("@echo off\n");
        script_content.push_str("setlocal enabledelayedexpansion\n");
        script_content.push_str("REM fnva environment switch script (CMD)\n");
        script_content.push_str("REM Usage: fnva-env.bat [environment_name]\n\n");

        // 获取环境名称参数
        script_content.push_str("set \"TARGET_ENV=%1\"\n");
        script_content.push_str("if \"%TARGET_ENV%\"==\"\" set \"TARGET_ENV=");
        if let Some(current) = &config.current_java_env {
            script_content.push_str(current);
        }
        script_content.push_str("\"\n\n");

        // 如果没有目标环境，提示错误
        script_content.push_str("if \"%TARGET_ENV%\"==\"\" (\n");
        script_content
            .push_str("    echo Error: No environment specified and no current environment\n");
        script_content.push_str("    exit /b 1\n");
        script_content.push_str(")\n\n");

        // 环境配置（简化版，只支持当前环境）
        let target_env = if !env_name.is_empty() {
            env_name
        } else if let Some(current) = &config.current_java_env {
            current
        } else {
            return Err("No available environment".to_string());
        };

        if let Some(env) = config.get_java_env(target_env) {
            // 简化的批处理脚本，专注于环境变量设置
            script_content.push_str(&format!("set \"JAVA_HOME={}\"\n", env.java_home));
            script_content.push_str(&format!("set \"PATH={}\\bin;%PATH%\"\n", env.java_home));

            // 输出和验证
            script_content.push_str(&format!(
                "echo Successfully switched to Java environment: {target_env}\n"
            ));
            script_content.push_str(&format!("echo JAVA_HOME: {}\n", env.java_home));
            script_content.push_str(&format!(
                "if exist \"{}\\bin\\java.exe\" (\n",
                env.java_home
            ));
            script_content.push_str("    echo Verifying Java version:\n");
            script_content.push_str(&format!(
                "    \"{}\\bin\\java.exe\" --version\n",
                env.java_home
            ));
            script_content.push_str(") else (\n");
            script_content.push_str("    echo Warning: Java executable not found\n");
            script_content.push_str(")\n");
        } else {
            return Err(format!("Environment not found: {target_env}"));
        }

        Ok(script_content)
    }

    /// 生成 Shell 集成安装脚本
    pub fn generate_shell_integration() -> Result<String, String> {
        let script_dir = dirs::home_dir()
            .ok_or_else(|| "Cannot get user home directory".to_string())?
            .join(".fnva");

        // PowerShell Profile 集成
        let ps_profile_script = Self::generate_powershell_profile_integration(&script_dir)?;
        let ps_profile_path = script_dir.join("powershell-integration.ps1");
        std::fs::write(&ps_profile_path, ps_profile_script)
            .map_err(|e| format!("Failed to write PowerShell integration script: {e}"))?;

        // CMD 集成
        let cmd_integration_script = Self::generate_cmd_integration(&script_dir)?;
        let cmd_integration_path = script_dir.join("cmd-integration.bat");
        std::fs::write(&cmd_integration_path, cmd_integration_script)
            .map_err(|e| format!("Failed to write CMD integration script: {e}"))?;

        Ok(format!(
            "Shell integration script generated\n\
            \n\
            PowerShell integration:\n\
            Add the following to your PowerShell profile:\n\
            {}\n\
            \n\
            CMD integration:\n\
            Add the following to your startup script:\n\
            {}\n\
            \n\
            Or run the integration script directly:\n\
            PowerShell: powershell -ExecutionPolicy Bypass -File {}\n\
            CMD: {}\n\
            \n\
            After installing, you can use:\n\
            fnva java use jdk21  # switch to the jdk21 environment",
            std::fs::read_to_string(&ps_profile_path).unwrap_or_default(),
            std::fs::read_to_string(&cmd_integration_path).unwrap_or_default(),
            ps_profile_path.display(),
            cmd_integration_path.display()
        ))
    }

    /// 生成 PowerShell Profile 集成脚本
    fn generate_powershell_profile_integration(_script_dir: &Path) -> Result<String, String> {
        let script_content = r#"# fnva PowerShell 集成
# 添加到你的 PowerShell Profile 中

$fnvaScript = Join-Path $env:USERPROFILE ".fnva\fnva-env.ps1"
if (Test-Path $fnvaScript) {
    # 创建 fnva 函数别名
    function fnva {
        param($Name)
        if ($args.Count -eq 0) {
            # 如果没有参数，显示当前环境
            & "$env:USERPROFILE\.fnva\fnva-env.ps1"
        } else {
            # 切换环境
            & "$env:USERPROFILE\.fnva\fnva-env.ps1" -EnvName $args[0]
        }
    }

    Write-Host "fnva environment switching loaded" -ForegroundColor Green
    Write-Host "Use 'fnva java use jdk21' to switch Java environments" -ForegroundColor Cyan
} else {
    Write-Warning "fnva environment script not found; run: fnva env"
}"#
        .to_string();

        Ok(script_content)
    }

    /// 生成 CMD 集成脚本
    fn generate_cmd_integration(_script_dir: &Path) -> Result<String, String> {
        let script_content = r#"@echo off
REM fnva CMD 集成
REM 添加到你的环境变量或启动脚本中

set "fnvaScript=%USERPROFILE%\.fnva\fnva-env.bat"
if exist "%fnvaScript%" (
    REM 创建 fnva.bat 调用脚本
    echo @echo off > "%USERPROFILE%\.fnva\fnva-call.bat"
    echo call "%fnvaScript%" %%* >> "%USERPROFILE%\.fnva\fnva-call.bat"

    REM 添加到 PATH
    set "PATH=%USERPROFILE%\.fnva;%PATH%"

    echo fnva environment switching loaded
    echo Use 'fnva java use jdk21' to switch Java environments
) else (
    echo Warning: fnva environment script not found; run: fnva env
)"#
        .to_string();

        Ok(script_content)
    }

    /// 创建命令行包装器
    pub fn create_command_wrapper(env_name: &str) -> Result<String, String> {
        let mut config = Config::load()?;

        let env = config
            .get_java_env(env_name)
            .ok_or_else(|| format!("Java environment '{env_name}' not found"))?
            .clone(); // 提前克隆以避免借用冲突

        // 验证路径
        if !crate::utils::validate_java_home(&env.java_home) {
            return Err(format!("Invalid JAVA_HOME path: {}", env.java_home));
        }

        // 保存为当前激活环境
        config.set_current_java_env(env_name.to_string())?;
        config.save()?;

        // 设置当前进程环境变量
        std::env::set_var("JAVA_HOME", &env.java_home);

        let bin_path = format!("{}\\bin", env.java_home);
        if let Ok(current_path) = std::env::var("PATH") {
            let new_path = format!("{bin_path};{current_path}");
            std::env::set_var("PATH", new_path);
        }

        Ok(format!(
            "Activated Java environment: {} ({})\n\
            JAVA_HOME: {}\n\
            BIN directory: {}\n\
            \n\
            Tip: environment variables are active in the current session\n\
            Reopening the terminal will reactivate this environment",
            env_name, env.description, env.java_home, bin_path
        ))
    }
}
