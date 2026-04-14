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
            _ => {
                return Err(AppError::ScriptGeneration {
                    shell_type: "PowerShell".to_string(),
                    reason: format!("不支持的环境类型: {env_type:?}"),
                })
            }
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
            _ => {
                return Err(AppError::ScriptGeneration {
                    shell_type: "Bash".to_string(),
                    reason: format!("不支持的环境类型: {env_type:?}"),
                })
            }
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
            _ => {
                return Err(AppError::ScriptGeneration {
                    shell_type: "Fish".to_string(),
                    reason: format!("不支持的环境类型: {env_type:?}"),
                })
            }
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
            _ => {
                return Err(AppError::ScriptGeneration {
                    shell_type: "CMD".to_string(),
                    reason: format!("不支持的环境类型: {env_type:?}"),
                })
            }
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

# Remove existing Java paths from PATH first
$pathParts = $env:PATH -split ';'
$cleanPath = @()
foreach ($part in $pathParts) {
    if ($part -notmatch 'java' -and $part -notmatch 'jdk') {
        $cleanPath += $part
    }
}

# Set new JAVA_HOME and update PATH
$env:JAVA_HOME = "{{escape_backslash java_home}}"
$env:PATH = "{{escape_backslash java_bin}};" + ($cleanPath -join ';')

# Set fnva environment tracking
$env:FNVA_CURRENT_JAVA = "{{env_name}}"
$env:FNVA_ENV_TYPE = "Java"

# Verify the switch
Write-Host "[OK] Switched to Java environment: {{env_name}}" -ForegroundColor Green
Write-Host "[DIR] JAVA_HOME: $env:JAVA_HOME" -ForegroundColor Yellow
Write-Host "[INFO] Java Version:" -ForegroundColor Cyan
try {
    & "{{escape_backslash java_bin}}\\java.exe" -version 2>&1 | ForEach-Object { Write-Host "   $_" -ForegroundColor Gray }
} catch {
    Write-Host "   Failed to get Java version" -ForegroundColor Red
}
"#;

const POWERSHELL_INTEGRATION_TEMPLATE: &str = r#"
# PowerShell Integration Script for fnva
# Add this to your PowerShell Profile ($PROFILE)

# Auto-load default environments on startup
$fnvaAutoLoadDone = $false
function fnva-AutoLoadDefault {
    if ($fnvaAutoLoadDone) { return }
    $fnvaAutoLoadDone = $true

    $envsFile = "$env:USERPROFILE\.fnva\current_envs.toml"
    if ((Test-Path $envsFile) -and (Get-Command fnva -ErrorAction SilentlyContinue)) {
        $lines = Get-Content $envsFile -ErrorAction SilentlyContinue
        foreach ($line in $lines) {
            if ($line -notmatch '^\s*(\w+)\s*=\s*"([^"]*)"') { continue }
            $key = $Matches[1]
            $value = $Matches[2]
            if ([string]::IsNullOrWhiteSpace($value)) { continue }

            $envScript = & fnva $key use $value 2>$null
            if ($envScript) {
                Invoke-Expression $envScript
            }
        }
    }
}

function fnva-Integration {
    $envsFile = "$env:USERPROFILE\.fnva\current_envs.toml"
    if (-not (Test-Path $envsFile)) { return }

    $lines = Get-Content $envsFile -ErrorAction SilentlyContinue
    foreach ($line in $lines) {
            if ($line -notmatch '^\s*(\w+)\s*=\s*"([^"]*)"') { continue }
            $key = $Matches[1]
            $value = $Matches[2]
            if ([string]::IsNullOrWhiteSpace($value)) { continue }

            $marker = "FNVA_RESTORED_$($key.ToUpper())"
            if (Get-ChildItem env:$marker -ErrorAction SilentlyContinue) {
                if ([Environment]::GetEnvironmentVariable($marker) -eq $value) { continue }
            }

            $envScript = & fnva $key use $value 2>$null
            if ($envScript) {
                Invoke-Expression $envScript
                [Environment]::SetEnvironmentVariable($marker, $value)
            }
        }
}

# Run autoload on startup
fnva-AutoLoadDefault

# Hook into PowerShell prompt
$OriginalPrompt = $function:prompt
function prompt {
    fnva-Integration
    & $OriginalPrompt
}

Write-Host "🚀 fnva PowerShell integration loaded" -ForegroundColor Green"#;

const BASH_JAVA_SWITCH_TEMPLATE: &str = r#"
#!/bin/bash
# Bash/Zsh Java Environment Switch - {{env_name}}
# Generated by fnva

# Remove existing Java paths from PATH
NEW_PATH=""
IFS=':' read -ra ADDR <<< "$PATH"
for i in "${ADDR[@]}"; do
    if [[ ! "$i" =~ java && ! "$i" =~ jdk ]]; then
        if [[ -z "$NEW_PATH" ]]; then
            NEW_PATH="$i"
        else
            NEW_PATH="$NEW_PATH:$i"
        fi
    fi
done

# Set new JAVA_HOME and update PATH
export JAVA_HOME="{{java_home}}"
export PATH="{{java_bin}}:$NEW_PATH"

# Set fnva environment tracking
export FNVA_CURRENT_JAVA="{{env_name}}"
export FNVA_ENV_TYPE="Java"

# Verify the switch
echo "[OK] Switched to Java environment: {{env_name}}"
echo "[DIR] JAVA_HOME: $JAVA_HOME"
echo "[INFO] Java Version:"
if [ -x "{{java_bin}}/java" ]; then
    "{{java_bin}}/java" -version 2>&1 | head -n 1 | sed 's/^/   /'
else
    echo "   Failed to get Java version"
fi

# Add to shell history
echo "fnva java use {{env_name}}" >> ~/.fnva/history 2>/dev/null || true
"#;

const BASH_INTEGRATION_TEMPLATE: &str = r#"
#!/bin/bash
# Bash/Zsh Integration Script for fnva
# Add this to your ~/.bashrc or ~/.zshrc

# Auto-load default environments on startup
_fnva_autoload_done=false
fnva_autoload_default() {
    if [[ $_fnva_autoload_done == "true" ]]; then
        return
    fi
    _fnva_autoload_done=true

    # Load default environments from current_envs.toml
    local envs_file="$HOME/.fnva/current_envs.toml"
    if [[ -f "$envs_file" ]] && command -v fnva >/dev/null 2>&1; then
        while IFS='=' read -r key value; do
            key=$(echo "$key" | tr -d '[:space:]')
            value=$(echo "$value" | tr -d '[:space:]' | tr -d '"')
            [[ -z "$value" ]] && continue
            local env_script
            env_script=$(fnva "$key" use "$value" 2>/dev/null)
            if [[ -n "$env_script" ]]; then
                eval "$env_script"
            fi
        done < "$envs_file"
    fi
}

fnva_hook() {
    local envs_file="$HOME/.fnva/current_envs.toml"
    if [[ ! -f "$envs_file" ]]; then return; fi

    while IFS='=' read -r key value; do
        key=$(echo "$key" | tr -d '[:space:]')
        value=$(echo "$value" | tr -d '[:space:]' | tr -d '"')
        [[ -z "$value" ]] && continue

        local marker="FNVA_RESTORED_$(echo "$key" | tr '[:lower:]' '[:upper:]')"
        local current_val
        current_val=$(eval echo "\"\$$marker\"" 2>/dev/null)
        [[ "$current_val" == "$value" ]] && continue

        local env_script
        env_script=$(fnva "$key" use "$value" 2>/dev/null)
        if [[ -n "$env_script" ]]; then
            eval "$env_script"
            export "$marker"="$value"
        fi
    done < "$envs_file"
}

# Run autoload on startup
fnva_autoload_default

# Hook into prompt
fnva_update_prompt() {
    fnva_hook

    # Show current environment in prompt (optional)
    local fnva_prompt=""
    if [[ -n "$FNVA_CURRENT_JAVA" ]]; then
        fnva_prompt="[Java: $FNVA_CURRENT_JAVA]"
    elif [[ -n "$FNVA_CURRENT_LLM" ]]; then
        fnva_prompt="[LLM: $FNVA_CURRENT_LLM]"
    elif [[ -n "$FNVA_CURRENT_CC" ]]; then
        fnva_prompt="[CC: $FNVA_CURRENT_CC]"
    fi

    if [[ -n "$fnva_prompt" ]]; then
        echo -e "\033[90m$fnva_prompt\033[0m"
    fi
}

# Hook into different shells
if [[ -n "$BASH_VERSION" ]]; then
    # Bash
    PROMPT_COMMAND="fnva_hook; $PROMPT_COMMAND"
elif [[ -n "$ZSH_VERSION" ]]; then
    # Zsh
    precmd_functions=(fnva_hook "${precmd_functions[@]}")
fi

echo "🚀 fnva Bash/Zsh integration loaded""#;

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
Write-Host "[OK] Switched to {{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}} environment: {{env_name}}" -ForegroundColor Green

{{#if config.anthropic_auth_token}}
Write-Host "[KEY] Anthropic Auth Token: [SET]" -ForegroundColor Yellow
{{/if}}

{{#if config.anthropic_base_url}}
Write-Host "[URL] Base URL: {{config.anthropic_base_url}}" -ForegroundColor Yellow
{{/if}}
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
echo "[OK] Switched to {{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}} environment: {{env_name}}"

{{#if config.anthropic_auth_token}}
echo "[KEY] Anthropic Auth Token: [SET]"
{{/if}}

{{#if config.anthropic_base_url}}
echo "[URL] Base URL: {{config.anthropic_base_url}}"
{{/if}}
"#;

const FISH_JAVA_SWITCH_TEMPLATE: &str = r#"
# Fish Java Environment Switch - {{env_name}}
# Generated by fnva

# Remove existing Java paths from PATH
set -gx JAVA_HOME "{{java_home}}"
set -gx PATH "{{java_bin}}" $PATH

# Set fnva environment tracking
set -gx FNVA_CURRENT_JAVA "{{env_name}}"
set -gx FNVA_ENV_TYPE "Java"

# Verify the switch
echo "[OK] Switched to Java environment: {{env_name}}"
echo "[DIR] JAVA_HOME: $JAVA_HOME"
echo "[INFO] Java Version:"
if test -x "{{java_bin}}/java"
    "{{java_bin}}/java" -version 2>&1 | head -n 1 | sed 's/^/   /'
else
    echo "   Failed to get Java version"
end

# Add to command history
echo "fnva java use {{env_name}}" >> ~/.fnva/history 2>/dev/null || true
"#;

const FISH_INTEGRATION_TEMPLATE: &str = r#"
# Fish Integration Script for fnva
# Add this to your ~/.config/fish/config.fish

# Auto-load default environments on startup
set -g _fnva_autoload_done false
function fnva_autoload_default
    if test $_fnva_autoload_done = true
        return
    end
    set -g _fnva_autoload_done true

    set envs_file "$HOME/.fnva/current_envs.toml"
    if test -f "$envs_file"; and command -v fnva >/dev/null 2>&1
        for line in (cat "$envs_file")
            set -l match (string match -r '^\s*(\w+)\s*=\s*"([^"]*)"',$line)
            test (count $match) -ge 3; or continue
            set key $match[2]
            set value $match[3]
test -n "$value"; or continue

            set env_script (fnva $key use $value 2>/dev/null)
            if test -n "$env_script"
                eval "$env_script"
            end
        end
    end
end

function fnva_hook --on-event fish_prompt
    set envs_file "$HOME/.fnva/current_envs.toml"
    test -f "$envs_file"; or return

    for line in (cat "$envs_file")
        # Parse "key = "value"" lines
        set -l match (string match -r '^\s*(\w+)\s*=\s*"([^"]*)"',$line)
        test (count $match) -ge 3; or continue
        set key $match[2]
        set value $match[3]
test -n "$value"; or continue

        set marker "FNVA_RESTORED_"(string upper $key)
        # Check if marker env var already has this value
        if set -q $marker
            set -l current_val $$marker
test "$current_val" = "$value"; and continue
        end

        set env_script (fnva $key use $value 2>/dev/null)
        if test -n "$env_script"
            eval "$env_script"
            set -gx $marker $value
        end
    end
end

# Run autoload on startup
fnva_autoload_default

# Function to show current environment in prompt
function fnva_prompt
    set -l fnva_prompt ""
    if set -q FNVA_CURRENT_JAVA
        set fnva_prompt "[Java: $FNVA_CURRENT_JAVA]"
    else if set -q FNVA_CURRENT_LLM
        set fnva_prompt "[LLM: $FNVA_CURRENT_LLM]"
    else if set -q FNVA_CURRENT_CC
        set fnva_prompt "[CC: $FNVA_CURRENT_CC]"
    end

    if test -n "$fnva_prompt"
        set_color 666666
        echo -n "$fnva_prompt"
        set_color normal
    end
end

echo "🚀 fnva Fish integration loaded""#;

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
echo "[OK] Switched to {{#if (eq env_type "Cc")}}Claude Code (CC){{else}}LLM{{/if}} environment: {{env_name}}"

{{#if config.anthropic_auth_token}}
echo "[KEY] Anthropic Auth Token: [SET]"
{{/if}}

{{#if config.anthropic_base_url}}
echo "[URL] Base URL: {{config.anthropic_base_url}}"
{{/if}}
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
        // 测试 helper 函数
        let template = "{{escape_backslash path}}";
        handlebars::Handlebars::new()
            .render_template(template, &json!({"path": "C:\\Test"}))
            .unwrap();
    }
}
