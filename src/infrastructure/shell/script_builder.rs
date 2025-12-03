#![allow(dead_code, deprecated)] // 兼容旧接口：保留但避免警告，推荐使用 ScriptGenerator

use crate::core::environment_manager::EnvironmentType;
use crate::infrastructure::shell::script_factory::ScriptGenerator;
use crate::infrastructure::shell::ShellType;
use std::collections::HashMap;

/// Shell 脚本构建器（向后兼容的包装器）
#[deprecated(note = "使用 ScriptGenerator 替代")]
pub struct ScriptBuilder {
    generator: ScriptGenerator,
}

impl ScriptBuilder {
    /// 创建新的脚本构建器
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            generator: ScriptGenerator::new()?,
        })
    }

    /// 使用默认生成器
    pub fn default() -> Self {
        Self {
            generator: ScriptGenerator::default(),
        }
    }
}

impl ScriptBuilder {
    /// 构建环境切换脚本
    pub fn build_switch_script(
        &self,
        env_type: EnvironmentType,
        env_name: &str,
        config: &serde_json::Value,
        shell_type: ShellType,
    ) -> Result<String, String> {
        self.generator
            .generate_switch_script(env_type, env_name, config, Some(shell_type))
            .map_err(|e| e.to_string())
    }

    /// 构建集成脚本
    pub fn build_integration_script(
        &self,
        current_envs: &HashMap<EnvironmentType, String>,
        shell_type: ShellType,
    ) -> Result<String, String> {
        self.generator
            .generate_integration_script(current_envs, Some(shell_type))
            .map_err(|e| e.to_string())
    }

    /// 向后兼容的同步方法（静态版本）
    pub fn build_switch_script_static(
        env_type: EnvironmentType,
        env_name: &str,
        config: &serde_json::Value,
        shell_type: ShellType,
    ) -> Result<String, String> {
        let builder = Self::default();
        builder.build_switch_script(env_type, env_name, config, shell_type)
    }

    /// 向后兼容的同步方法（静态版本）
    pub fn build_integration_script_static(
        current_envs: &HashMap<EnvironmentType, String>,
        shell_type: ShellType,
    ) -> Result<String, String> {
        let builder = Self::default();
        builder.build_integration_script(current_envs, shell_type)
    }

    /// 构建 PowerShell 切换脚本
    fn build_powershell_switch_script(
        env_type: EnvironmentType,
        env_name: &str,
        config: &serde_json::Value,
    ) -> Result<String, String> {
        let mut script = String::new();

        match env_type {
            EnvironmentType::Java => {
                let java_home = config
                    .get("java_home")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing java_home in config")?;

                // Remove existing Java paths from PATH first
                script.push_str("# Remove existing Java paths from PATH\r\n");
                script.push_str("$pathParts = $env:PATH -split ';'\r\n");
                script.push_str("$cleanPath = @()\r\n");
                script.push_str("foreach ($part in $pathParts) {\r\n");
                script.push_str("    if ($part -notmatch 'java' -and $part -notmatch 'jdk') {\r\n");
                script.push_str("        $cleanPath += $part\r\n");
                script.push_str("    }\r\n");
                script.push_str("}\r\n");

                // Set new JAVA_HOME and update PATH
                script.push_str(&format!(
                    "$env:JAVA_HOME = \"{}\"\r\n",
                    java_home.replace('\\', "\\\\")
                ));

                let bin_path = format!("{}\\bin", java_home);
                script.push_str(&format!(
                    "$env:PATH = \"{};\" + ($cleanPath -join ';')\r\n",
                    bin_path.replace('\\', "\\\\")
                ));

                // Verify the switch
                script.push_str(&format!(
                    "Write-Host \"Switched to Java environment: {}\" -ForegroundColor Green\r\n",
                    env_name
                ));
                script.push_str(
                    "Write-Host \"JAVA_HOME: $env:JAVA_HOME\" -ForegroundColor Yellow\r\n",
                );
            }
            EnvironmentType::Llm | EnvironmentType::Cc => {
                // Check if this is an Anthropic/GLM_CC environment
                let is_anthropic = config.get("anthropic_auth_token").is_some();

                if is_anthropic {
                    // Anthropic/GLM_CC environment variables
                    if let Some(auth_token) =
                        config.get("anthropic_auth_token").and_then(|v| v.as_str())
                    {
                        script
                            .push_str(&format!("$env:ANTHROPIC_AUTH_TOKEN = \"{}\"\n", auth_token));
                    }

                    if let Some(base_url) =
                        config.get("anthropic_base_url").and_then(|v| v.as_str())
                    {
                        script.push_str(&format!("$env:ANTHROPIC_BASE_URL = \"{}\"\n", base_url));
                    }

                    if let Some(timeout) = config.get("api_timeout_ms").and_then(|v| v.as_str()) {
                        script.push_str(&format!("$env:API_TIMEOUT_MS = \"{}\"\n", timeout));
                    }

                    if let Some(disable_traffic) =
                        config.get("claude_code_disable_nonessential_traffic")
                    {
                        if disable_traffic.as_u64().unwrap_or(0) == 1 {
                            script.push_str("$env:CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = 1\n");
                        }
                    }

                    // Set default Sonnet model if specified
                    if let Some(default_model) = config
                        .get("default_model")
                        .and_then(|v| v.as_str())
                    {
                        script.push_str(&format!(
                            "$env:ANTHROPIC_DEFAULT_SONNET_MODEL = \"{}\"\n",
                            default_model
                        ));
                    }

                    // Note: Removed OPENAI_API_KEY setting for CC environments
                    // CC (Claude Code) environments should not set OpenAI variables
                } else {
                    // OpenAI environment variables (original implementation)
                    let api_key = config
                        .get("api_key")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing api_key in config")?;

                    script.push_str(&format!("$env:OPENAI_API_KEY = \"{}\"\n", api_key));

                    if let Some(model) = config.get("model").and_then(|v| v.as_str()) {
                        script.push_str(&format!("$env:OPENAI_MODEL = \"{}\"\n", model));
                    }

                    if let Some(base_url) = config.get("base_url").and_then(|v| v.as_str()) {
                        script.push_str(&format!("$env:OPENAI_BASE_URL = \"{}\"\n", base_url));
                    }
                }
            }
            _ => {
                return Err(format!("Environment type {:?} not yet supported", env_type));
            }
        }

        Ok(script)
    }

    /// 构建 Bash/Zsh 切换脚本
    fn build_bash_switch_script(
        env_type: EnvironmentType,
        env_name: &str,
        config: &serde_json::Value,
    ) -> Result<String, String> {
        let mut script = String::new();

        match env_type {
            EnvironmentType::Java => {
                let java_home = config
                    .get("java_home")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing java_home in config")?;

                // Remove existing Java paths from PATH first
                script.push_str("# Remove existing Java paths from PATH\n");
                script.push_str("clean_path=$(echo \"$PATH\" | tr ':' '\\n' | grep -v java | grep -v jdk | tr '\\n' ':' | sed 's/:$//')\n");

                script.push_str(&format!("export JAVA_HOME=\"{}\"\n", java_home));
                script.push_str(&format!("export PATH=\"{}\\bin:$clean_path\"\n", java_home));

                // Verify the switch
                script.push_str(&format!(
                    "echo \"Switched to Java environment: {}\"\n",
                    env_name
                ));
                script.push_str("echo \"JAVA_HOME: $JAVA_HOME\"\n");
            }
            EnvironmentType::Llm | EnvironmentType::Cc => {
                // Check if this is an Anthropic/GLM_CC environment
                let is_anthropic = config.get("anthropic_auth_token").is_some();

                if is_anthropic {
                    // Anthropic/GLM_CC environment variables
                    if let Some(auth_token) =
                        config.get("anthropic_auth_token").and_then(|v| v.as_str())
                    {
                        script
                            .push_str(&format!("export ANTHROPIC_AUTH_TOKEN=\"{}\"\n", auth_token));
                    }

                    if let Some(base_url) =
                        config.get("anthropic_base_url").and_then(|v| v.as_str())
                    {
                        script.push_str(&format!("export ANTHROPIC_BASE_URL=\"{}\"\n", base_url));
                    }

                    if let Some(timeout) = config.get("api_timeout_ms").and_then(|v| v.as_str()) {
                        script.push_str(&format!("export API_TIMEOUT_MS=\"{}\"\n", timeout));
                    }

                    if let Some(disable_traffic) =
                        config.get("claude_code_disable_nonessential_traffic")
                    {
                        if disable_traffic.as_u64().unwrap_or(0) == 1 {
                            script.push_str("export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1\n");
                        }
                    }

                    // Set default Sonnet model if specified
                    if let Some(default_model) = config
                        .get("default_model")
                        .and_then(|v| v.as_str())
                    {
                        script.push_str(&format!(
                            "export ANTHROPIC_DEFAULT_SONNET_MODEL=\"{}\"\n",
                            default_model
                        ));
                    }

                    // Note: Removed OPENAI_API_KEY setting for CC environments
                    // CC (Claude Code) environments should not set OpenAI variables
                } else {
                    // OpenAI environment variables (original implementation)
                    let api_key = config
                        .get("api_key")
                        .and_then(|v| v.as_str())
                        .ok_or("Missing api_key in config")?;

                    script.push_str(&format!("export OPENAI_API_KEY=\"{}\"\n", api_key));

                    if let Some(model) = config.get("model").and_then(|v| v.as_str()) {
                        script.push_str(&format!("export OPENAI_MODEL=\"{}\"\n", model));
                    }

                    if let Some(base_url) = config.get("base_url").and_then(|v| v.as_str()) {
                        script.push_str(&format!("export OPENAI_BASE_URL=\"{}\"\n", base_url));
                    }
                }
            }
            _ => {
                return Err(format!("Environment type {:?} not yet supported", env_type));
            }
        }

        Ok(script)
    }

    /// 构建 Fish 切换脚本
    fn build_fish_switch_script(
        env_type: EnvironmentType,
        env_name: &str,
        config: &serde_json::Value,
    ) -> Result<String, String> {
        let mut script = String::new();

        match env_type {
            EnvironmentType::Java => {
                let java_home = config
                    .get("java_home")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing java_home in config")?;

                // Remove existing Java paths from PATH first
                script.push_str("# Remove existing Java paths from PATH\n");
                script.push_str("set clean_path (echo $PATH | tr ' ' '\\n' | grep -v java | grep -v jdk | tr '\\n' ' ' | string trim)\n");

                script.push_str(&format!("set -gx JAVA_HOME \"{}\"\n", java_home));
                script.push_str(&format!(
                    "set -gx PATH \"{}\\bin\" $clean_path\n",
                    java_home
                ));

                // Verify the switch
                script.push_str(&format!(
                    "echo \"Switched to Java environment: {}\"\n",
                    env_name
                ));
                script.push_str("echo \"JAVA_HOME: $JAVA_HOME\"\n");
            }
            EnvironmentType::Llm => {
                let api_key = config
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing api_key in config")?;

                script.push_str(&format!("set -gx OPENAI_API_KEY \"{}\"\n", api_key));
            }
            EnvironmentType::Cc => {
                // CC environments use Anthropic variables, not OpenAI
                // No OpenAI variables should be set for Claude Code environments
            }
            _ => {
                return Err(format!("Environment type {:?} not yet supported", env_type));
            }
        }

        Ok(script)
    }

    /// 构建 CMD 切换脚本
    fn build_cmd_switch_script(
        env_type: EnvironmentType,
        env_name: &str,
        config: &serde_json::Value,
    ) -> Result<String, String> {
        let mut script = String::new();

        match env_type {
            EnvironmentType::Java => {
                let java_home = config
                    .get("java_home")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing java_home in config")?;

                // Remove existing Java paths from PATH first
                script.push_str("@echo off\n");
                script.push_str("REM Remove existing Java paths from PATH\n");
                script.push_str("setlocal enabledelayedexpansion\n");
                script.push_str("set \"clean_path=\"\n");
                script.push_str("for %%i in (\"%PATH:;= \"%\") do (\n");
                script.push_str("    echo %%~i | findstr /i java >nul\n");
                script.push_str("    if errorlevel 1 echo %%~i | findstr /i jdk >nul\n");
                script.push_str("    if errorlevel 1 (\n");
                script.push_str("        if defined clean_path (\n");
                script.push_str("            set \"clean_path=!clean_path!;%%~i\"\n");
                script.push_str("        ) else (\n");
                script.push_str("            set \"clean_path=%%~i\"\n");
                script.push_str("        )\n");
                script.push_str("    )\n");
                script.push_str(")\n");

                script.push_str(&format!("set \"JAVA_HOME={}\"\n", java_home));
                script.push_str(&format!("set \"PATH={}\\bin;!clean_path!\"\n", java_home));

                // Verify the switch
                script.push_str(&format!(
                    "echo Switched to Java environment: {}\n",
                    env_name
                ));
                script.push_str("echo JAVA_HOME: %JAVA_HOME%\n");
            }
            EnvironmentType::Llm => {
                let api_key = config
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing api_key in config")?;

                script.push_str(&format!("set OPENAI_API_KEY={}\n", api_key));

                if let Some(model) = config.get("model").and_then(|v| v.as_str()) {
                    script.push_str(&format!("set OPENAI_MODEL={}\n", model));
                }

                if let Some(base_url) = config.get("base_url").and_then(|v| v.as_str()) {
                    script.push_str(&format!("set OPENAI_BASE_URL={}\n", base_url));
                }
            }
            EnvironmentType::Cc => {
                // CC environments use Anthropic variables, not OpenAI
                // No OpenAI variables should be set for Claude Code environments
            }
            _ => {
                return Err(format!("Environment type {:?} not yet supported", env_type));
            }
        }

        Ok(script)
    }

    /// 构建 PowerShell 集成脚本 - 类似 fnm 的简洁方案
    fn build_powershell_integration_script(
        _current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, String> {
        let username = std::env::var("USERNAME").unwrap_or_else(|_| "Administrator".to_string());

        // 创建真正的 fnm 风格集成
        let mut script = String::new();

        script.push_str("# fnva PowerShell Integration - fnm style\n");
        script.push_str(&format!("# Add this to your PowerShell profile:\n"));
        script.push_str(&format!(
            "# C:\\Users\\{}\\Documents\\PowerShell\\Microsoft.PowerShell_profile.ps1\n\n",
            username
        ));

        // 核心的 fnm 风格集成逻辑
        script.push_str("# Override fnva command for automatic environment switching\n");
        script.push_str("function fnva {\n");
        script.push_str("    param(\n");
        script.push_str("        [Parameter(Position=0, ValueFromRemainingArguments)]\n");
        script.push_str("        [string[]]$Args\n");
        script.push_str("    )\n\n");

        // 处理 Java 环境切换命令
        script.push_str("    # Handle Java environment switching specially\n");
        script.push_str(
            "    if ($Args.Count -ge 2 -and $Args[0] -eq \"java\" -and $Args[1] -eq \"use\") {\n",
        );
        script.push_str("        $envName = $Args[2]\n");
        script.push_str("        if ($envName) {\n");
        script.push_str("            # Generate and execute PowerShell script\n");
        script.push_str(
            "            $script = & $PSCommandPath java use $envName --shell powershell 2>$null\n",
        );
        script.push_str("            if ($script) {\n");
        script.push_str("                try {\n");
        script.push_str("                    Invoke-Expression $script\n");
        script.push_str("                    Write-Host \"✅ Switched to Java: $envName\" -ForegroundColor Green\n");
        script.push_str("                } catch {\n");
        script.push_str("                    Write-Error \"Failed to switch Java environment: $($_.Exception.Message)\"\n");
        script.push_str("                }\n");
        script.push_str("            }\n");
        script.push_str("        }\n");
        script.push_str("    } else {\n");
        script.push_str("        # For all other commands, pass through to original fnva\n");
        script.push_str("        & $PSCommandPath @Args\n");
        script.push_str("    }\n");
        script.push_str("}\n\n");

        // 添加便捷别名
        script.push_str("# Convenient aliases for common operations\n");
        script.push_str("function jfn { fnva java use $args }\n");
        script.push_str("function jls { fnva java list }\n");
        script.push_str("function jcur { fnva java current }\n");

        Ok(script)
    }

    /// 构建 Bash/Zsh 集成脚本
    fn build_bash_integration_script(
        _current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, String> {
        let script = r#"# fnva Bash/Zsh Integration
# Add this to your ~/.bashrc or ~/.zshrc

fnva_hook() {
    local env_file="$HOME/.fnva/current_env"
    if [[ -f "$env_file" ]]; then
        local current_env=$(cat "$env_file" 2>/dev/null | tr -d '[:space:]')
        if [[ -n "$current_env" && "$FNVA_CURRENT_ENV" != "$current_env" ]]; then
            # Apply environment using fnva command
            eval "$(fnva env current --shell bash 2>/dev/null)"
            export FNVA_CURRENT_ENV="$current_env"
        fi
    fi
}

# Hook into PROMPT_COMMAND (Bash) or precmd (Zsh)
if [[ -n "$BASH_VERSION" ]]; then
    PROMPT_COMMAND="fnva_hook; $PROMPT_COMMAND"
elif [[ -n "$ZSH_VERSION" ]]; then
    precmd_functions=(fnva_hook "${precmd_functions[@]}")
fi

echo "🚀 fnva Bash/Zsh integration loaded"
"#;

        Ok(script.to_string())
    }

    /// 构建 Fish 集成脚本
    fn build_fish_integration_script(
        _current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, String> {
        let script = r#"# fnva Fish Integration
# Add this to your ~/.config/fish/config.fish

function fnva_hook --on-variable PWD
    set env_file "$HOME/.fnva/current_env"
    if test -f "$env_file"
        set current_env (cat "$env_file" 2>/dev/null | string trim)
        if test -n "$current_env"; and test "$FNVA_CURRENT_ENV" != "$current_env"
            # Apply environment using fnva command
            fnva env current --shell fish | source
            set -gx FNVA_CURRENT_ENV "$current_env"
        end
    end
end

echo "🚀 fnva Fish integration loaded"
"#;

        Ok(script.to_string())
    }

    /// 构建 CMD 集成脚本
    fn build_cmd_integration_script(
        _current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, String> {
        let script = r#"@echo off
REM fnva CMD Integration
REM Add this to your startup script

REM Check and apply fnva environments
set "env_file=%USERPROFILE%\.fnva\current_env"
if exist "%env_file%" (
    set /p current_env=<"%env_file%"
    set "current_env=%current_env: =%"
    if defined current_env (
        if not "%FNVA_CURRENT_ENV%"=="%current_env%" (
            REM Apply environment using fnva command
            for /f "tokens=*" %%i in ('fnva env current --shell cmd 2^>nul') do %%i
            set "FNVA_CURRENT_ENV=%current_env%"
        )
    )
)

echo 🚀 fnva CMD integration loaded
"#;

        Ok(script.to_string())
    }
}
