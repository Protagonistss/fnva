use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::environment_manager::EnvironmentType;
use crate::error::AppError;
use crate::infrastructure::shell::ShellType;

/// 脚本生成策略接口
pub trait ScriptGenerationStrategy: Send + Sync {
    /// 生成环境切换脚本
    fn generate_switch_script(
        &self,
        env_type: EnvironmentType,
        env_name: &str,
        config: &Value,
    ) -> Result<String, AppError>;

    /// 生成集成脚本
    fn generate_integration_script(
        &self,
        current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, AppError>;

    /// 获取Shell类型
    fn shell_type(&self) -> ShellType;

    /// 支持环境变量设置
    fn supports_env_vars(&self) -> bool {
        true
    }
}

/// 模板引擎包装器
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    /// 创建新的模板引擎
    pub fn new() -> Result<Self, AppError> {
        let mut handlebars = Handlebars::new();

        // 注册助手函数
        handlebars.register_helper("escape_backslash", Box::new(handlebars_escape_backslash));
        handlebars.register_helper("to_upper", Box::new(handlebars_to_upper));
        handlebars.register_helper("path_join", Box::new(handlebars_path_join));
        handlebars.register_helper("env_var_name", Box::new(handlebars_env_var_name));

        // 注册模板
        Self::register_templates(&mut handlebars)?;

        Ok(Self { handlebars })
    }

    /// 注册所有模板
    fn register_templates(handlebars: &mut Handlebars) -> Result<(), AppError> {
        // PowerShell 模板
        handlebars
            .register_template_string("powershell_java_switch", POWERSHELL_JAVA_SWITCH_TEMPLATE)?;
        handlebars
            .register_template_string("powershell_cc_switch", POWERSHELL_CC_SWITCH_TEMPLATE)?;
        handlebars
            .register_template_string("powershell_integration", POWERSHELL_INTEGRATION_TEMPLATE)?;

        // Bash/Zsh 模板
        handlebars.register_template_string("bash_java_switch", BASH_JAVA_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("bash_cc_switch", BASH_CC_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("bash_integration", BASH_INTEGRATION_TEMPLATE)?;

        // Fish 模板
        handlebars.register_template_string("fish_java_switch", FISH_JAVA_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("fish_cc_switch", FISH_CC_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("fish_integration", FISH_INTEGRATION_TEMPLATE)?;

        // CMD 模板
        handlebars.register_template_string("cmd_java_switch", CMD_JAVA_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("cmd_cc_switch", CMD_CC_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("cmd_integration", CMD_INTEGRATION_TEMPLATE)?;

        // Maven 模板（各 shell）
        handlebars.register_template_string(
            "powershell_maven_switch",
            POWERSHELL_MAVEN_SWITCH_TEMPLATE,
        )?;
        handlebars.register_template_string("bash_maven_switch", BASH_MAVEN_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("fish_maven_switch", FISH_MAVEN_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("cmd_maven_switch", CMD_MAVEN_SWITCH_TEMPLATE)?;

        Ok(())
    }

    /// 渲染模板
    pub fn render(&self, template_name: &str, data: &Value) -> Result<String, AppError> {
        self.handlebars
            .render(template_name, data)
            .map_err(|e| AppError::Serialization(format!("模板渲染失败: {e}")))
    }
}

/// PowerShell 脚本生成策略
pub struct PowerShellStrategy {
    template_engine: Arc<TemplateEngine>,
}

impl PowerShellStrategy {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            template_engine: Arc::new(TemplateEngine::new()?),
        })
    }
}

impl ScriptGenerationStrategy for PowerShellStrategy {
    fn generate_switch_script(
        &self,
        env_type: EnvironmentType,
        env_name: &str,
        config: &Value,
    ) -> Result<String, AppError> {
        let template_name = match env_type {
            EnvironmentType::Java => "powershell_java_switch",
            EnvironmentType::Maven => "powershell_maven_switch",
            EnvironmentType::Cc => "powershell_cc_switch",
        };

        let mut data = json!({
            "env_name": env_name,
            "env_type": env_type,
            "config": config,
        });

        // 添加特定环境类型的数据
        if env_type == EnvironmentType::Java {
            if let Some(java_home) = config.get("java_home").and_then(|v| v.as_str()) {
                data["java_home"] = json!(java_home);
                data["java_bin"] = json!(format!("{}\\bin", java_home));
            }
        } else if env_type == EnvironmentType::Maven {
            if let Some(maven_home) = config.get("maven_home").and_then(|v| v.as_str()) {
                data["maven_home"] = json!(maven_home);
                data["maven_bin"] = json!(format!("{}\\bin", maven_home));
            }
        }

        self.template_engine.render(template_name, &data)
    }

    fn generate_integration_script(
        &self,
        current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, AppError> {
        let data = json!({
            "current_envs": current_envs,
            "shell_type": "PowerShell",
        });

        self.template_engine.render("powershell_integration", &data)
    }

    fn shell_type(&self) -> ShellType {
        ShellType::PowerShell
    }
}

/// Bash/Zsh 脚本生成策略
pub struct BashStrategy {
    template_engine: Arc<TemplateEngine>,
}

impl BashStrategy {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            template_engine: Arc::new(TemplateEngine::new()?),
        })
    }
}

impl ScriptGenerationStrategy for BashStrategy {
    fn generate_switch_script(
        &self,
        env_type: EnvironmentType,
        env_name: &str,
        config: &Value,
    ) -> Result<String, AppError> {
        let template_name = match env_type {
            EnvironmentType::Java => "bash_java_switch",
            EnvironmentType::Maven => "bash_maven_switch",
            EnvironmentType::Cc => "bash_cc_switch",
        };

        let mut data = json!({
            "env_name": env_name,
            "env_type": env_type,
            "config": config,
        });

        if env_type == EnvironmentType::Java {
            if let Some(java_home) = config.get("java_home").and_then(|v| v.as_str()) {
                data["java_home"] = json!(java_home);
                data["java_bin"] = json!(format!("{}/bin", java_home));
            }
        } else if env_type == EnvironmentType::Maven {
            if let Some(maven_home) = config.get("maven_home").and_then(|v| v.as_str()) {
                data["maven_home"] = json!(maven_home);
                data["maven_bin"] = json!(format!("{}/bin", maven_home));
            }
        }

        self.template_engine.render(template_name, &data)
    }

    fn generate_integration_script(
        &self,
        current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, AppError> {
        let data = json!({
            "current_envs": current_envs,
            "shell_type": "Bash/Zsh",
        });

        self.template_engine.render("bash_integration", &data)
    }

    fn shell_type(&self) -> ShellType {
        ShellType::Bash
    }
}

/// Fish 脚本生成策略
pub struct FishStrategy {
    template_engine: Arc<TemplateEngine>,
}

impl FishStrategy {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            template_engine: Arc::new(TemplateEngine::new()?),
        })
    }
}

impl ScriptGenerationStrategy for FishStrategy {
    fn generate_switch_script(
        &self,
        env_type: EnvironmentType,
        env_name: &str,
        config: &Value,
    ) -> Result<String, AppError> {
        let template_name = match env_type {
            EnvironmentType::Java => "fish_java_switch",
            EnvironmentType::Maven => "fish_maven_switch",
            EnvironmentType::Cc => "fish_cc_switch",
        };

        let mut data = json!({
            "env_name": env_name,
            "env_type": env_type,
            "config": config,
        });

        if env_type == EnvironmentType::Java {
            if let Some(java_home) = config.get("java_home").and_then(|v| v.as_str()) {
                data["java_home"] = json!(java_home);
                data["java_bin"] = json!(format!("{}/bin", java_home));
            }
        } else if env_type == EnvironmentType::Maven {
            if let Some(maven_home) = config.get("maven_home").and_then(|v| v.as_str()) {
                data["maven_home"] = json!(maven_home);
                data["maven_bin"] = json!(format!("{}/bin", maven_home));
            }
        }

        self.template_engine.render(template_name, &data)
    }

    fn generate_integration_script(
        &self,
        current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, AppError> {
        let data = json!({
            "current_envs": current_envs,
            "shell_type": "Fish",
        });

        self.template_engine.render("fish_integration", &data)
    }

    fn shell_type(&self) -> ShellType {
        ShellType::Fish
    }
}

/// CMD 脚本生成策略
pub struct CmdStrategy {
    template_engine: Arc<TemplateEngine>,
}

impl CmdStrategy {
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            template_engine: Arc::new(TemplateEngine::new()?),
        })
    }
}

impl ScriptGenerationStrategy for CmdStrategy {
    fn generate_switch_script(
        &self,
        env_type: EnvironmentType,
        env_name: &str,
        config: &Value,
    ) -> Result<String, AppError> {
        let template_name = match env_type {
            EnvironmentType::Java => "cmd_java_switch",
            EnvironmentType::Maven => "cmd_maven_switch",
            EnvironmentType::Cc => "cmd_cc_switch",
        };

        let mut data = json!({
            "env_name": env_name,
            "env_type": env_type,
            "config": config,
        });

        if env_type == EnvironmentType::Java {
            if let Some(java_home) = config.get("java_home").and_then(|v| v.as_str()) {
                data["java_home"] = json!(java_home);
                data["java_bin"] = json!(format!("{}\\bin", java_home));
            }
        } else if env_type == EnvironmentType::Maven {
            if let Some(maven_home) = config.get("maven_home").and_then(|v| v.as_str()) {
                data["maven_home"] = json!(maven_home);
                data["maven_bin"] = json!(format!("{}\\bin", maven_home));
            }
        }

        self.template_engine.render(template_name, &data)
    }

    fn generate_integration_script(
        &self,
        current_envs: &HashMap<EnvironmentType, String>,
    ) -> Result<String, AppError> {
        let data = json!({
            "current_envs": current_envs,
            "shell_type": "CMD",
        });

        self.template_engine.render("cmd_integration", &data)
    }

    fn shell_type(&self) -> ShellType {
        ShellType::Cmd
    }
}

/// Handlebars 助手函数
fn handlebars_escape_backslash(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        let value = param.value().as_str().unwrap_or("");
        let escaped = value.replace('\\', "\\\\");
        out.write(&escaped)?;
    }
    Ok(())
}

fn handlebars_to_upper(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        let value = param.value().as_str().unwrap_or("");
        out.write(&value.to_uppercase())?;
    }
    Ok(())
}

fn handlebars_path_join(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(path1) = h.param(0).and_then(|p| p.value().as_str()) {
        if let Some(path2) = h.param(1).and_then(|p| p.value().as_str()) {
            let joined = format!("{}{}{}", path1, std::path::MAIN_SEPARATOR, path2);
            out.write(&joined)?;
        }
    }
    Ok(())
}

fn handlebars_env_var_name(
    h: &handlebars::Helper,
    _: &handlebars::Handlebars,
    _: &handlebars::Context,
    _: &mut handlebars::RenderContext,
    out: &mut dyn handlebars::Output,
) -> handlebars::HelperResult {
    if let Some(param) = h.param(0) {
        let value = param.value().as_str().unwrap_or("");
        let env_name = value.to_uppercase().replace(['-', '.'], "_");
        out.write(&format!("FNVA_{env_name}"))?;
    }
    Ok(())
}

// 模板常量定义
const POWERSHELL_JAVA_SWITCH_TEMPLATE: &str = r#"
# PowerShell Java Environment Switch - {{env_name}}
# Generated by fnva

# 设置UTF-8编码以正确显示中文
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Console]::OutputEncoding

# Clean previous fnva-managed path, then set new JAVA_HOME
if ($env:FNVA_JAVA_BIN) {
    $env:PATH = ($env:PATH -split ';' | Where-Object { $_ -ne $env:FNVA_JAVA_BIN }) -join ';'
}
$env:FNVA_JAVA_BIN = "{{escape_backslash java_bin}}"
$env:JAVA_HOME = "{{escape_backslash java_home}}"
$env:PATH = "{{escape_backslash java_bin}};" + $env:PATH

# Set fnva environment tracking
$env:FNVA_CURRENT_JAVA = "{{env_name}}"
$env:FNVA_ENV_TYPE = "Java"

# Verify the switch
if (-not $env:_FNVA_QUIET) {
    Write-Host "Switched to Java environment: {{env_name}}" -ForegroundColor Green
    Write-Host "JAVA_HOME: $env:JAVA_HOME" -ForegroundColor Yellow
    Write-Host "Java Version:" -ForegroundColor Cyan
    try {
        & "{{escape_backslash java_bin}}\\java.exe" -version 2>&1 | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
    } catch {
        Write-Host "   Failed to get Java version" -ForegroundColor Red
    }
}
"#;

const POWERSHELL_INTEGRATION_TEMPLATE: &str = r#"
# fnva environment setup (fnva env --shell powershell | Out-String | Invoke-Expression)

# --- Auto-restore on startup ---
$fnvaAutoLoadDone = $false
function fnva-AutoLoadDefault {
    if ($fnvaAutoLoadDone) { return }
    $fnvaAutoLoadDone = $true

    $envsFile = "$env:USERPROFILE\.fnva\state\current_envs.toml"
    if ((Test-Path $envsFile) -and (Get-Command fnva -ErrorAction SilentlyContinue)) {
        $restored = @()
        $lines = Get-Content $envsFile -ErrorAction SilentlyContinue
        foreach ($line in $lines) {
            if ($line -notmatch '^\s*(\w+)\s*=\s*"([^"]*)"') { continue }
            $key = $Matches[1]
            $value = $Matches[2]
            if ([string]::IsNullOrWhiteSpace($value)) { continue }
            $env:_FNVA_QUIET = "1"
            $envScript = (& fnva.cmd $key use $value 2>$null) -join "`n"
            if ($envScript) { Invoke-Expression $envScript; $restored += "$key $value" }
            Remove-Item Env:\_FNVA_QUIET
        }
        if ($restored.Count -gt 0) {
            Write-Host -NoNewline "✓ " -ForegroundColor Green
            Write-Host "fnva active: $($restored -join ', ')"
        }
    }
}

fnva-AutoLoadDefault

# --- Shell wrapper (auto-source on use) ---
function fnva {
    if ($args.Count -ge 2 -and ($args[0] -eq "java" -or $args[0] -eq "cc" -or $args[0] -eq "maven") -and ($args[1] -eq "use")) {
        $tempFile = Join-Path $env:TEMP ("fnva_script_" + (Get-Random) + ".ps1")
        try {
            & fnva.cmd @args 2>&1 | Out-File -FilePath $tempFile -Encoding UTF8
            $content = Get-Content $tempFile -Raw -Encoding UTF8
            if ($content -match '\$env:' -or $content -match 'Write-Host') {
                . $tempFile
            } else {
                $content
            }
        } finally {
            if (Test-Path $tempFile) { Remove-Item $tempFile -ErrorAction SilentlyContinue }
        }
    } else {
        & fnva.cmd @args
    }
}
"#;

const BASH_JAVA_SWITCH_TEMPLATE: &str = r#"
#!/bin/bash
# Bash/Zsh Java Environment Switch - {{env_name}}
# Generated by fnva

# Clean previous fnva-managed path, then set new JAVA_HOME
if [ -n "${FNVA_JAVA_BIN:-}" ]; then
    PATH="${PATH//${FNVA_JAVA_BIN}:/}"
fi
export FNVA_JAVA_BIN="{{java_bin}}"
export JAVA_HOME="{{java_home}}"
export PATH="$FNVA_JAVA_BIN:$PATH"

# Set fnva environment tracking
export FNVA_CURRENT_JAVA="{{env_name}}"
export FNVA_ENV_TYPE="Java"

# Verify the switch
if [[ -z "$_FNVA_QUIET" ]]; then
    echo "Switched to Java environment: {{env_name}}"
    echo "JAVA_HOME: $JAVA_HOME"
    echo "Java Version:"
    if [ -x "{{java_bin}}/java" ]; then
        "{{java_bin}}/java" -version 2>&1 | head -n 1 | sed 's/^/   /'
    else
        echo "   Failed to get Java version"
    fi
fi

"#;

const BASH_INTEGRATION_TEMPLATE: &str = r#"
#!/bin/bash
# fnva environment setup (eval "$(fnva env --shell bash)")

# --- Auto-restore on startup ---
_fnva_autoload_done=false
fnva_autoload_default() {
    if [[ $_fnva_autoload_done == "true" ]]; then return; fi
    _fnva_autoload_done=true

    local envs_file="$HOME/.fnva/state/current_envs.toml"
    if [[ -f "$envs_file" ]] && command -v fnva >/dev/null 2>&1; then
        local _restored=""
        while IFS='=' read -r key value; do
            key=$(echo "$key" | tr -d '[:space:]')
            value=$(echo "$value" | tr -d '[:space:]' | tr -d '"')
            [[ -z "$value" ]] && continue
            _FNVA_QUIET=1 eval "$(command fnva "$key" use "$value" 2>/dev/null)" >/dev/null 2>&1
            unset _FNVA_QUIET
            if [[ -z "$_restored" ]]; then
                _restored="${key} ${value}"
            else
                _restored="$_restored, ${key} ${value}"
            fi
        done < "$envs_file"
        if [[ -n "$_restored" ]]; then
            printf '\033[32m✓\033[0m fnva active: %s\n' "$_restored"
        fi
    fi
}

fnva_autoload_default

# --- Shell wrapper (auto-source on use) ---
fnva() {
    if [[ $# -ge 2 && ("$1" == "java" || "$1" == "cc" || "$1" == "maven") && "$2" == "use" ]]; then
        local temp_file
        temp_file="$(mktemp)"
        command fnva "$@" > "$temp_file"
        source "$temp_file"
        rm -f "$temp_file"
    else
        command fnva "$@"
    fi
}
"#;

// 其他模板常量...
const POWERSHELL_CC_SWITCH_TEMPLATE: &str = r#"
# PowerShell Claude Code Environment Switch - {{env_name}}
# Generated by fnva

# 设置UTF-8编码以正确显示中文
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Console]::OutputEncoding

{{#if config.anthropic_auth_token}}
# Anthropic/GLM-CC environment
$env:ANTHROPIC_AUTH_TOKEN = "{{config.anthropic_auth_token}}"
{{/if}}

{{#if config.anthropic_base_url}}
$env:ANTHROPIC_BASE_URL = "{{config.anthropic_base_url}}"
{{/if}}

{{#if config.opus_model}}
$env:ANTHROPIC_DEFAULT_OPUS_MODEL = "{{config.opus_model}}"
{{/if}}

{{#if config.sonnet_model}}
$env:ANTHROPIC_DEFAULT_SONNET_MODEL = "{{config.sonnet_model}}"
{{/if}}

{{#if config.haiku_model}}
$env:ANTHROPIC_DEFAULT_HAIKU_MODEL = "{{config.haiku_model}}"
{{/if}}

# Set fnva environment tracking
$env:FNVA_CURRENT_CC = "{{env_name}}"
$env:FNVA_ENV_TYPE = "CC"

# Claude Code specific settings
{{#if config.anthropic_auth_token}}
$env:CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"
$env:API_TIMEOUT_MS = "{{config.api_timeout_ms}}"
{{/if}}

# Verify the switch
if (-not $env:_FNVA_QUIET) {
    Write-Host "Switched to Claude Code (CC) environment: {{env_name}}" -ForegroundColor Green

    {{#if config.anthropic_auth_token}}
    Write-Host "Anthropic Auth Token: [SET]" -ForegroundColor Yellow
    {{/if}}

    {{#if config.anthropic_base_url}}
    Write-Host "Base URL: {{config.anthropic_base_url}}" -ForegroundColor Yellow
    {{/if}}
}
"#;

const BASH_CC_SWITCH_TEMPLATE: &str = r#"
#!/bin/bash
# Bash/Zsh Claude Code Environment Switch - {{env_name}}
# Generated by fnva

{{#if config.anthropic_auth_token}}
# Anthropic/GLM-CC environment
export ANTHROPIC_AUTH_TOKEN="{{config.anthropic_auth_token}}"
{{/if}}

{{#if config.anthropic_base_url}}
export ANTHROPIC_BASE_URL="{{config.anthropic_base_url}}"
{{/if}}

{{#if config.opus_model}}
export ANTHROPIC_DEFAULT_OPUS_MODEL="{{config.opus_model}}"
{{/if}}

{{#if config.sonnet_model}}
export ANTHROPIC_DEFAULT_SONNET_MODEL="{{config.sonnet_model}}"
{{/if}}

{{#if config.haiku_model}}
export ANTHROPIC_DEFAULT_HAIKU_MODEL="{{config.haiku_model}}"
{{/if}}

# Set fnva environment tracking
export FNVA_CURRENT_CC="{{env_name}}"
export FNVA_ENV_TYPE="CC"

# Claude Code specific settings
{{#if config.anthropic_auth_token}}
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1"
export API_TIMEOUT_MS="{{config.api_timeout_ms}}"
{{/if}}

# Verify the switch
if [[ -z "$_FNVA_QUIET" ]]; then
    echo "Switched to Claude Code (CC) environment: {{env_name}}"

    {{#if config.anthropic_auth_token}}
    echo "Anthropic Auth Token: [SET]"
    {{/if}}

    {{#if config.anthropic_base_url}}
    echo "Base URL: {{config.anthropic_base_url}}"
    {{/if}}
fi
"#;

const FISH_JAVA_SWITCH_TEMPLATE: &str = r#"
# Fish Java Environment Switch - {{env_name}}
# Generated by fnva

# Clean previous fnva-managed path, then set new JAVA_HOME
if set -q FNVA_JAVA_BIN
    set -gx PATH (string match -v "^$FNVA_JAVA_BIN\$" $PATH)
end
set -gx FNVA_JAVA_BIN "{{java_bin}}"
set -gx JAVA_HOME "{{java_home}}"
set -gx PATH "{{java_bin}}" $PATH

# Set fnva environment tracking
set -gx FNVA_CURRENT_JAVA "{{env_name}}"
set -gx FNVA_ENV_TYPE "Java"

# Verify the switch
if not set -q _FNVA_QUIET
    echo "Switched to Java environment: {{env_name}}"
    echo "JAVA_HOME: $JAVA_HOME"
    echo "Java Version:"
    if test -x "{{java_bin}}/java"
        "{{java_bin}}/java" -version 2>&1 | head -n 1 | sed 's/^/   /'
    else
        echo "   Failed to get Java version"
    end
end

"#;

const FISH_INTEGRATION_TEMPLATE: &str = r#"
# fnva environment setup (fnva env --shell fish | source)

# --- Auto-restore on startup ---
set -g _fnva_autoload_done false
function fnva_autoload_default
    if test $_fnva_autoload_done = true; return; end
    set -g _fnva_autoload_done true

    set envs_file "$HOME/.fnva/state/current_envs.toml"
    if test -f "$envs_file"; and command -v fnva >/dev/null 2>&1
        set -l _restored
        for line in (cat "$envs_file")
            set -l match (string match -r '^\s*(\w+)\s*=\s*"([^"]*)"' -- $line)
            test (count $match) -ge 3; or continue
            set key $match[2]
            set value $match[3]
            test -n "$value"; or continue
            set _t (mktemp)
            _FNVA_QUIET=1 command fnva $key use $value > $_t 2>/dev/null
            source $_t >/dev/null 2>&1
            rm -f $_t
            set -e _FNVA_QUIET
            set -a _restored "$key $value"
        end
        if set -q _restored[1]
            printf '\033[32m✓\033[0m fnva active: %s\n' (string join ', ' $_restored)
        end
    end
end

fnva_autoload_default

# --- Shell wrapper (auto-source on use) ---
function fnva
    if test (count $argv) -ge 2; and string match -q -r "^(java|cc|maven)$" $argv[1]; and test $argv[2] = "use"
        set temp_file (mktemp)
        command fnva $argv > $temp_file
        source $temp_file
        rm -f $temp_file
    else
        command fnva $argv
    end
end
"#;

const FISH_CC_SWITCH_TEMPLATE: &str = r#"
# Fish Claude Code Environment Switch - {{env_name}}
# Generated by fnva

{{#if config.anthropic_auth_token}}
# Anthropic/GLM-CC environment
set -gx ANTHROPIC_AUTH_TOKEN "{{config.anthropic_auth_token}}"
{{/if}}

{{#if config.anthropic_base_url}}
set -gx ANTHROPIC_BASE_URL "{{config.anthropic_base_url}}"
{{/if}}

{{#if config.opus_model}}
set -gx ANTHROPIC_DEFAULT_OPUS_MODEL "{{config.opus_model}}"
{{/if}}

{{#if config.sonnet_model}}
set -gx ANTHROPIC_DEFAULT_SONNET_MODEL "{{config.sonnet_model}}"
{{/if}}

{{#if config.haiku_model}}
set -gx ANTHROPIC_DEFAULT_HAIKU_MODEL "{{config.haiku_model}}"
{{/if}}

# Set fnva environment tracking
set -gx FNVA_CURRENT_CC "{{env_name}}"
set -gx FNVA_ENV_TYPE "CC"

# Claude Code specific settings
{{#if config.anthropic_auth_token}}
set -gx CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC "1"
set -gx API_TIMEOUT_MS "{{config.api_timeout_ms}}"
{{/if}}

# Verify the switch
if not set -q _FNVA_QUIET
    echo "Switched to Claude Code (CC) environment: {{env_name}}"

    {{#if config.anthropic_auth_token}}
    echo "Anthropic Auth Token: [SET]"
    {{/if}}

    {{#if config.anthropic_base_url}}
    echo "Base URL: {{config.anthropic_base_url}}"
    {{/if}}
end
"#;

const CMD_JAVA_SWITCH_TEMPLATE: &str = r#"
@echo off
REM CMD Java Environment Switch - {{env_name}}
REM Generated by fnva

REM Set new JAVA_HOME
set "JAVA_HOME={{escape_backslash java_home}}"

REM Set fnva environment tracking
set "FNVA_CURRENT_JAVA={{env_name}}"
set "FNVA_ENV_TYPE=Java"

REM Update PATH to include Java bin
if defined FNVA_JAVA_BIN call set "PATH=%%PATH:%FNVA_JAVA_BIN%;=%%"
set "FNVA_JAVA_BIN={{escape_backslash java_bin}}"
set "PATH=%FNVA_JAVA_BIN%;%PATH%"

REM Verify the switch
echo Switched to Java environment: {{env_name}}
echo JAVA_HOME: %JAVA_HOME%
echo Java Version:
if exist "{{escape_backslash java_bin}}\java.exe" (
    "{{escape_backslash java_bin}}\java.exe" -version 2>&1
) else (
    echo    Failed to get Java version
)

"#;

const CMD_INTEGRATION_TEMPLATE: &str = r#"
@echo off
REM CMD Integration Script for fnva
REM Add this to your startup script
setlocal enabledelayedexpansion

REM Auto-restore environments from current_envs.toml
set "envs_file=%USERPROFILE%\.fnva\state\current_envs.toml"
if exist "%envs_file%" (
    for /f "usebackq tokens=1,* delims==" %%a in ("%envs_file%") do (
        set "env_key=%%a"
        set "env_val=%%b"
        set "env_val=!env_val: =!"
        set "env_val=!env_val:"=!"
        if not "!env_val!"=="" (
            where fnva >nul 2>&1
            if !errorlevel! equ 0 (
                for /f "tokens=*" %%s in ('fnva !env_key! use !env_val! 2^>nul') do (
                    %%s
                )
            )
        )
    )
)

echo fnva CMD integration loaded"#;

const CMD_CC_SWITCH_TEMPLATE: &str = r#"
@echo off
REM CMD Claude Code Environment Switch - {{env_name}}
REM Generated by fnva

{{#if config.anthropic_auth_token}}
REM Anthropic/GLM-CC environment
set "ANTHROPIC_AUTH_TOKEN={{config.anthropic_auth_token}}"
{{/if}}

{{#if config.anthropic_base_url}}
set "ANTHROPIC_BASE_URL={{config.anthropic_base_url}}"
{{/if}}

{{#if config.opus_model}}
set "ANTHROPIC_DEFAULT_OPUS_MODEL={{config.opus_model}}"
{{/if}}

{{#if config.sonnet_model}}
set "ANTHROPIC_DEFAULT_SONNET_MODEL={{config.sonnet_model}}"
{{/if}}

{{#if config.haiku_model}}
set "ANTHROPIC_DEFAULT_HAIKU_MODEL={{config.haiku_model}}"
{{/if}}

REM Set fnva environment tracking
set "FNVA_CURRENT_CC={{env_name}}"
set "FNVA_ENV_TYPE=CC"

REM Claude Code specific settings
{{#if config.anthropic_auth_token}}
set "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1"
set "API_TIMEOUT_MS={{config.api_timeout_ms}}"
{{/if}}

REM Verify the switch
echo Switched to Claude Code (CC) environment: {{env_name}}

{{#if config.anthropic_auth_token}}
echo Anthropic Auth Token: [SET]
{{/if}}

{{#if config.anthropic_base_url}}
echo Base URL: {{config.anthropic_base_url}}"
{{/if}}

"#;

const POWERSHELL_MAVEN_SWITCH_TEMPLATE: &str = r#"
# PowerShell Maven Environment Switch - {{env_name}}
# Generated by fnva

# 设置UTF-8编码以正确显示中文
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding = [System.Console]::OutputEncoding

# Clean previous fnva-managed path, then set new MAVEN_HOME / M2_HOME
if ($env:FNVA_MAVEN_BIN) {
    $env:PATH = ($env:PATH -split ';' | Where-Object { $_ -ne $env:FNVA_MAVEN_BIN }) -join ';'
}
$env:FNVA_MAVEN_BIN = "{{escape_backslash maven_bin}}"
$env:MAVEN_HOME = "{{escape_backslash maven_home}}"
$env:M2_HOME = "{{escape_backslash maven_home}}"
$env:PATH = "{{escape_backslash maven_bin}};" + $env:PATH

# Set fnva environment tracking
$env:FNVA_CURRENT_MAVEN = "{{env_name}}"
$env:FNVA_ENV_TYPE = "Maven"

# Verify the switch
if (-not $env:_FNVA_QUIET) {
    Write-Host "Switched to Maven environment: {{env_name}}" -ForegroundColor Green
    Write-Host "MAVEN_HOME: $env:MAVEN_HOME" -ForegroundColor Yellow
    Write-Host "Maven Version:" -ForegroundColor Cyan
    try {
        & "{{escape_backslash maven_bin}}\\mvn.cmd" -v 2>&1 | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
    } catch {
        Write-Host "   Failed to get Maven version" -ForegroundColor Red
    }
}
"#;

const BASH_MAVEN_SWITCH_TEMPLATE: &str = r#"
#!/bin/bash
# Bash/Zsh Maven Environment Switch - {{env_name}}
# Generated by fnva

# Clean previous fnva-managed path, then set new MAVEN_HOME / M2_HOME
if [ -n "${FNVA_MAVEN_BIN:-}" ]; then
    PATH="${PATH//${FNVA_MAVEN_BIN}:/}"
fi
export FNVA_MAVEN_BIN="{{maven_bin}}"
export MAVEN_HOME="{{maven_home}}"
export M2_HOME="{{maven_home}}"
export PATH="$FNVA_MAVEN_BIN:$PATH"

# Set fnva environment tracking
export FNVA_CURRENT_MAVEN="{{env_name}}"
export FNVA_ENV_TYPE="Maven"

# Verify the switch
if [[ -z "$_FNVA_QUIET" ]]; then
    echo "Switched to Maven environment: {{env_name}}"
    echo "MAVEN_HOME: $MAVEN_HOME"
    echo "Maven Version:"
    if [ -x "{{maven_bin}}/mvn" ]; then
        "{{maven_bin}}/mvn" -v 2>&1 | head -n 1 | sed 's/^/   /'
    else
        echo "   Failed to get Maven version"
    fi
fi

"#;

const FISH_MAVEN_SWITCH_TEMPLATE: &str = r#"
# Fish Maven Environment Switch - {{env_name}}
# Generated by fnva

# Clean previous fnva-managed path, then set new MAVEN_HOME / M2_HOME
if set -q FNVA_MAVEN_BIN
    set -gx PATH (string match -v "^$FNVA_MAVEN_BIN\$" $PATH)
end
set -gx FNVA_MAVEN_BIN "{{maven_bin}}"
set -gx MAVEN_HOME "{{maven_home}}"
set -gx M2_HOME "{{maven_home}}"
set -gx PATH "{{maven_bin}}" $PATH

# Set fnva environment tracking
set -gx FNVA_CURRENT_MAVEN "{{env_name}}"
set -gx FNVA_ENV_TYPE "Maven"

# Verify the switch
if not set -q _FNVA_QUIET
    echo "Switched to Maven environment: {{env_name}}"
    echo "MAVEN_HOME: $MAVEN_HOME"
    echo "Maven Version:"
    if test -x "{{maven_bin}}/mvn"
        "{{maven_bin}}/mvn" -v 2>&1 | head -n 1 | sed 's/^/   /'
    else
        echo "   Failed to get Maven version"
    end
end

"#;

const CMD_MAVEN_SWITCH_TEMPLATE: &str = r#"
@echo off
REM CMD Maven Environment Switch - {{env_name}}
REM Generated by fnva

REM Set new MAVEN_HOME / M2_HOME
set "MAVEN_HOME={{escape_backslash maven_home}}"
set "M2_HOME={{escape_backslash maven_home}}"

REM Set fnva environment tracking
set "FNVA_CURRENT_MAVEN={{env_name}}"
set "FNVA_ENV_TYPE=Maven"

REM Update PATH to include Maven bin
if defined FNVA_MAVEN_BIN call set "PATH=%%PATH:%FNVA_MAVEN_BIN%;=%%"
set "FNVA_MAVEN_BIN={{escape_backslash maven_bin}}"
set "PATH=%FNVA_MAVEN_BIN%;%PATH%"

REM Verify the switch
echo Switched to Maven environment: {{env_name}}
echo MAVEN_HOME: %MAVEN_HOME%
echo Maven Version:
if exist "{{escape_backslash maven_bin}}\mvn.cmd" (
    "{{escape_backslash maven_bin}}\mvn.cmd" -v 2>&1
) else (
    echo    Failed to get Maven version
)

"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::shell::ScriptGenerator;
    use serde_json::json;

    #[test]
    fn test_powershell_strategy() {
        let strategy = PowerShellStrategy::new().unwrap();
        assert_eq!(strategy.shell_type(), ShellType::PowerShell);
        assert!(strategy.supports_env_vars());

        let config = json!({
            "java_home": "C:\\Program Files\\Java\\jdk-17"
        });

        let script = strategy
            .generate_switch_script(EnvironmentType::Java, "jdk17", &config)
            .unwrap();

        assert!(script.contains("JAVA_HOME"));
        assert!(script.contains("jdk-17"));
    }

    #[test]
    fn test_bash_strategy() {
        let strategy = BashStrategy::new().unwrap();
        assert_eq!(strategy.shell_type(), ShellType::Bash);

        let config = json!({
            "java_home": "/usr/lib/jvm/java-17"
        });

        let script = strategy
            .generate_switch_script(EnvironmentType::Java, "jdk17", &config)
            .unwrap();

        assert!(script.contains("JAVA_HOME"));
        assert!(script.contains("export"));
    }

    #[test]
    fn test_template_engine() {
        // 通过 ScriptGenerator 测试自定义 helper 是否正常注册
        let generator = ScriptGenerator::new().unwrap();
        let config = json!({"java_home": "C:\\Program Files\\Java\\jdk17", "java_bin": "C:\\Program Files\\Java\\jdk17\\bin", "env_name": "jdk17"});
        let script = generator
            .generate_switch_script(
                EnvironmentType::Java,
                "jdk17",
                &config,
                Some(ShellType::PowerShell),
            )
            .unwrap();

        // escape_backslash 应将单反斜杠替换为双反斜杠
        assert!(
            script.contains("C:\\\\Program Files\\\\Java\\\\jdk17"),
            "escape_backslash helper should escape backslashes: {script}"
        );
    }

    #[test]
    fn test_maven_strategy() {
        let strategy = BashStrategy::new().unwrap();
        let config = json!({ "maven_home": "/home/user/.fnva/packages/maven/3.9.16" });
        let script = strategy
            .generate_switch_script(EnvironmentType::Maven, "mvn39", &config)
            .unwrap();
        assert!(
            script.contains("MAVEN_HOME"),
            "should set MAVEN_HOME: {script}"
        );
        assert!(script.contains("M2_HOME"), "should set M2_HOME: {script}");
        assert!(
            script.contains("mvn39"),
            "should include env name: {script}"
        );
        assert!(
            script.contains("packages/maven/3.9.16/bin"),
            "should derive maven_bin: {script}"
        );
    }
}
