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
            .register_template_string("powershell_llm_switch", POWERSHELL_LLM_SWITCH_TEMPLATE)?;
        handlebars
            .register_template_string("powershell_integration", POWERSHELL_INTEGRATION_TEMPLATE)?;

        // Bash/Zsh 模板
        handlebars.register_template_string("bash_java_switch", BASH_JAVA_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("bash_llm_switch", BASH_LLM_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("bash_integration", BASH_INTEGRATION_TEMPLATE)?;

        // Fish 模板
        handlebars.register_template_string("fish_java_switch", FISH_JAVA_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("fish_llm_switch", FISH_LLM_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("fish_integration", FISH_INTEGRATION_TEMPLATE)?;

        // CMD 模板
        handlebars.register_template_string("cmd_java_switch", CMD_JAVA_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("cmd_llm_switch", CMD_LLM_SWITCH_TEMPLATE)?;
        handlebars.register_template_string("cmd_integration", CMD_INTEGRATION_TEMPLATE)?;

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
            EnvironmentType::Llm | EnvironmentType::Cc => "powershell_llm_switch",
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
            EnvironmentType::Llm | EnvironmentType::Cc => "bash_llm_switch",
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
            EnvironmentType::Llm | EnvironmentType::Cc => "fish_llm_switch",
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
            EnvironmentType::Llm | EnvironmentType::Cc => "cmd_llm_switch",
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

# Set new JAVA_HOME and prepend to PATH
$env:JAVA_HOME = "{{escape_backslash java_home}}"
$env:PATH = "{{escape_backslash java_bin}};" + $env:PATH

# Set fnva environment tracking
$env:FNVA_CURRENT_JAVA = "{{env_name}}"
$env:FNVA_ENV_TYPE = "Java"

# Verify the switch
if (-not $env:_FNVA_QUIET) {
    Write-Host "[OK] Switched to Java environment: {{env_name}}" -ForegroundColor Green
    Write-Host "[DIR] JAVA_HOME: $env:JAVA_HOME" -ForegroundColor Yellow
    Write-Host "[INFO] Java Version:" -ForegroundColor Cyan
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

    $envsFile = "$env:USERPROFILE\.fnva\current_envs.toml"
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
            if ($envScript) { Invoke-Expression $envScript; $restored += $value }
            Remove-Item Env:\_FNVA_QUIET
        }
        if ($restored.Count -gt 0) {
            Write-Host "[fnva] restored: $($restored -join ' ')" -ForegroundColor DarkGray
        }
    }
}

fnva-AutoLoadDefault

# --- Shell wrapper (auto-source on use) ---
function fnva {
    if ($args.Count -ge 2 -and ($args[0] -eq "java" -or $args[0] -eq "llm" -or $args[0] -eq "cc") -and ($args[1] -eq "use")) {
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

# Set new JAVA_HOME and prepend to PATH
export JAVA_HOME="{{java_home}}"
export PATH="{{java_bin}}:$PATH"

# Set fnva environment tracking
export FNVA_CURRENT_JAVA="{{env_name}}"
export FNVA_ENV_TYPE="Java"

# Verify the switch
if [[ -z "$_FNVA_QUIET" ]]; then
    echo "[OK] Switched to Java environment: {{env_name}}"
    echo "[DIR] JAVA_HOME: $JAVA_HOME"
    echo "[INFO] Java Version:"
    if [ -x "{{java_bin}}/java" ]; then
        "{{java_bin}}/java" -version 2>&1 | head -n 1 | sed 's/^/   /'
    else
        echo "   Failed to get Java version"
    fi
fi

# Add to shell history
echo "fnva java use {{env_name}}" >> ~/.fnva/history 2>/dev/null || true
"#;

const BASH_INTEGRATION_TEMPLATE: &str = r#"
#!/bin/bash
# fnva environment setup (eval "$(fnva env --shell bash)")

# --- Auto-restore on startup ---
_fnva_autoload_done=false
fnva_autoload_default() {
    if [[ $_fnva_autoload_done == "true" ]]; then return; fi
    _fnva_autoload_done=true

    local envs_file="$HOME/.fnva/current_envs.toml"
    if [[ -f "$envs_file" ]] && command -v fnva >/dev/null 2>&1; then
        local _restored=""
        while IFS='=' read -r key value; do
            key=$(echo "$key" | tr -d '[:space:]')
            value=$(echo "$value" | tr -d '[:space:]' | tr -d '"')
            [[ -z "$value" ]] && continue
            local env_script
            env_script=$(_FNVA_QUIET=1 command fnva "$key" use "$value" 2>/dev/null)
            if [[ -n "$env_script" ]]; then
                _FNVA_QUIET=1 eval "$env_script"
                unset _FNVA_QUIET
                _restored="$_restored $value"
            fi
        done < "$envs_file"
        if [[ -n "$_restored" ]]; then
            echo "[fnva] restored:$_restored"
        fi
    fi
}

fnva_autoload_default

# --- Shell wrapper (auto-source on use) ---
fnva() {
    if [[ $# -ge 2 && ("$1" == "java" || "$1" == "llm" || "$1" == "cc") && "$2" == "use" ]]; then
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
const POWERSHELL_LLM_SWITCH_TEMPLATE: &str = r#"
# PowerShell LLM/CC Environment Switch - {{env_name}}
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
$env:FNVA_CURRENT_{{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}} = "{{env_name}}"
$env:FNVA_ENV_TYPE = "{{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}}"

# Claude Code specific settings
{{#if config.anthropic_auth_token}}
$env:CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC = "1"
$env:API_TIMEOUT_MS = "30000"
{{/if}}

# Verify the switch
if (-not $env:_FNVA_QUIET) {
    Write-Host "[OK] Switched to {{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}} environment: {{env_name}}" -ForegroundColor Green

    {{#if config.anthropic_auth_token}}
    Write-Host "[KEY] Anthropic Auth Token: [SET]" -ForegroundColor Yellow
    {{/if}}

    {{#if config.anthropic_base_url}}
    Write-Host "[URL] Base URL: {{config.anthropic_base_url}}" -ForegroundColor Yellow
    {{/if}}
}
"#;

const BASH_LLM_SWITCH_TEMPLATE: &str = r#"
#!/bin/bash
# Bash/Zsh LLM/CC Environment Switch - {{env_name}}
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
export FNVA_CURRENT_{{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}}="{{env_name}}"
export FNVA_ENV_TYPE="{{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}}"

# Claude Code specific settings
{{#if config.anthropic_auth_token}}
export CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC="1"
export API_TIMEOUT_MS="30000"
{{/if}}

# Verify the switch
if [[ -z "$_FNVA_QUIET" ]]; then
    echo "[OK] Switched to {{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}} environment: {{env_name}}"

    {{#if config.anthropic_auth_token}}
    echo "[KEY] Anthropic Auth Token: [SET]"
    {{/if}}

    {{#if config.anthropic_base_url}}
    echo "[URL] Base URL: {{config.anthropic_base_url}}"
    {{/if}}
fi
"#;

const FISH_JAVA_SWITCH_TEMPLATE: &str = r#"
# Fish Java Environment Switch - {{env_name}}
# Generated by fnva

# Set new JAVA_HOME and prepend to PATH
set -gx JAVA_HOME "{{java_home}}"
set -gx PATH "{{java_bin}}" $PATH

# Set fnva environment tracking
set -gx FNVA_CURRENT_JAVA "{{env_name}}"
set -gx FNVA_ENV_TYPE "Java"

# Verify the switch
if not set -q _FNVA_QUIET
    echo "[OK] Switched to Java environment: {{env_name}}"
    echo "[DIR] JAVA_HOME: $JAVA_HOME"
    echo "[INFO] Java Version:"
    if test -x "{{java_bin}}/java"
        "{{java_bin}}/java" -version 2>&1 | head -n 1 | sed 's/^/   /'
    else
        echo "   Failed to get Java version"
    end
end

# Add to command history
echo "fnva java use {{env_name}}" >> ~/.fnva/history 2>/dev/null || true
"#;

const FISH_INTEGRATION_TEMPLATE: &str = r#"
# fnva environment setup (fnva env --shell fish | source)

# --- Auto-restore on startup ---
set -g _fnva_autoload_done false
function fnva_autoload_default
    if test $_fnva_autoload_done = true; return; end
    set -g _fnva_autoload_done true

    set envs_file "$HOME/.fnva/current_envs.toml"
    if test -f "$envs_file"; and command -v fnva >/dev/null 2>&1
        set -l _restored
        for line in (cat "$envs_file")
            set -l match (string match -r '^\s*(\w+)\s*=\s*"([^"]*)"' -- $line)
            test (count $match) -ge 3; or continue
            set key $match[2]
            set value $match[3]
            test -n "$value"; or continue
            set env_script (_FNVA_QUIET=1 command fnva $key use $value 2>/dev/null)
            if test -n "$env_script"; _FNVA_QUIET=1 eval "$env_script"; set -e _FNVA_QUIET; set -a _restored $value; end
        end
        if test (count $_restored) -gt 0
            echo "[fnva] restored: "(string join ' ' $_restored)
        end
    end
end

fnva_autoload_default

# --- Shell wrapper (auto-source on use) ---
function fnva
    if test (count $argv) -ge 2; and string match -q -r "^(java|llm|cc)$" $argv[1]; and test $argv[2] = "use"
        set temp_file (mktemp)
        command fnva $argv > $temp_file
        source $temp_file
        rm -f $temp_file
    else
        command fnva $argv
    end
end
"#;

const FISH_LLM_SWITCH_TEMPLATE: &str = r#"
# Fish LLM/CC Environment Switch - {{env_name}}
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
set -gx FNVA_CURRENT_{{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}} "{{env_name}}"
set -gx FNVA_ENV_TYPE "{{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}}"

# Claude Code specific settings
{{#if config.anthropic_auth_token}}
set -gx CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC "1"
set -gx API_TIMEOUT_MS "30000"
{{/if}}

# Verify the switch
if not set -q _FNVA_QUIET
    echo "[OK] Switched to {{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}} environment: {{env_name}}"

    {{#if config.anthropic_auth_token}}
    echo "[KEY] Anthropic Auth Token: [SET]"
    {{/if}}

    {{#if config.anthropic_base_url}}
    echo "[URL] Base URL: {{config.anthropic_base_url}}"
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
set "PATH={{escape_backslash java_bin}};%PATH%"

REM Verify the switch
echo [OK] Switched to Java environment: {{env_name}}
echo [DIR] JAVA_HOME: %JAVA_HOME%
echo [INFO] Java Version:
if exist "{{escape_backslash java_bin}}\java.exe" (
    "{{escape_backslash java_bin}}\java.exe" -version 2>&1
) else (
    echo    Failed to get Java version
)

REM Add to history
echo fnva java use {{env_name}} >> "%USERPROFILE%\.fnva\history" 2>nul
"#;

const CMD_INTEGRATION_TEMPLATE: &str = r#"
@echo off
REM CMD Integration Script for fnva
REM Add this to your startup script
setlocal enabledelayedexpansion

REM Auto-restore environments from current_envs.toml
set "envs_file=%USERPROFILE%\.fnva\current_envs.toml"
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

const CMD_LLM_SWITCH_TEMPLATE: &str = r#"
@echo off
REM CMD LLM/CC Environment Switch - {{env_name}}
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
set "FNVA_CURRENT_{{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}}={{env_name}}"
set "FNVA_ENV_TYPE={{#if (eq env_type "Cc")}}CC{{else}}LLM{{/if}}"

REM Claude Code specific settings
{{#if config.anthropic_auth_token}}
set "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1"
set "API_TIMEOUT_MS=30000"
{{/if}}

REM Verify the switch
echo [OK] Switched to {{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}} environment: {{env_name}}

{{#if config.anthropic_auth_token}}
echo [KEY] Anthropic Auth Token: [SET]
{{/if}}

{{#if config.anthropic_base_url}}
echo [URL] Base URL: {{config.anthropic_base_url}}"
{{/if}}

REM Add to history
echo fnva {{#if (eq env_type "Cc")}}cc{{else}}llm{{/if}} use {{env_name}} >> "%USERPROFILE%\.fnva\history" 2>nul
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
            .generate_switch_script(EnvironmentType::Java, "jdk17", &config, Some(ShellType::PowerShell))
            .unwrap();

        // escape_backslash 应将单反斜杠替换为双反斜杠
        assert!(script.contains("C:\\\\Program Files\\\\Java\\\\jdk17"), "escape_backslash helper should escape backslashes: {script}");
    }
}
