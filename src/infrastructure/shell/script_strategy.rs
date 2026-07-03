use handlebars::Handlebars;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::core::environment_manager::EnvironmentType;
use crate::error::AppError;
use crate::infrastructure::shell::ShellType;

/// 根据环境配置构建最终的 MAVEN_OPTS 字符串。
/// 合并顺序：用户自定义 maven_opts → local_repo → settings_file。
/// 若三项均未设置则返回空字符串（模板中不会 export MAVEN_OPTS）。
fn build_maven_opts_value(config: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();

    // 用户自定义 JVM 参数（原样保留）
    if let Some(opts) = config.get("maven_opts").and_then(|v| v.as_str()) {
        if !opts.is_empty() {
            parts.push(opts.to_string());
        }
    }

    // 自定义本地仓库路径 → -Dmaven.repo.local=...
    if let Some(repo) = config.get("local_repo").and_then(|v| v.as_str()) {
        if !repo.is_empty() {
            parts.push(format!("-Dmaven.repo.local={repo}"));
        }
    }

    // 自定义 settings.xml → -s ...
    if let Some(settings) = config.get("settings_file").and_then(|v| v.as_str()) {
        if !settings.is_empty() {
            parts.push(format!("-s {settings}"));
        }
    }

    parts.join(" ")
}

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
            .map_err(|e| AppError::Serialization(format!("Template rendering failed: {e}")))
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
            // 构建最终的 MAVEN_OPTS（合并用户设置 + local_repo + settings_file）
            let opts_value = build_maven_opts_value(config);
            data["has_maven_opts"] = json!(!opts_value.is_empty());
            data["maven_opts_value"] = json!(opts_value);
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
            // 构建最终的 MAVEN_OPTS（合并用户设置 + local_repo + settings_file）
            let opts_value = build_maven_opts_value(config);
            data["has_maven_opts"] = json!(!opts_value.is_empty());
            data["maven_opts_value"] = json!(opts_value);
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
            // 构建最终的 MAVEN_OPTS（合并用户设置 + local_repo + settings_file）
            let opts_value = build_maven_opts_value(config);
            data["has_maven_opts"] = json!(!opts_value.is_empty());
            data["maven_opts_value"] = json!(opts_value);
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
            // 构建最终的 MAVEN_OPTS（合并用户设置 + local_repo + settings_file）
            let opts_value = build_maven_opts_value(config);
            data["has_maven_opts"] = json!(!opts_value.is_empty());
            data["maven_opts_value"] = json!(opts_value);
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
const POWERSHELL_JAVA_SWITCH_TEMPLATE: &str = include_str!("templates/powershell_java_switch.hbs");

const POWERSHELL_INTEGRATION_TEMPLATE: &str = include_str!("templates/powershell_integration.hbs");

const BASH_JAVA_SWITCH_TEMPLATE: &str = include_str!("templates/bash_java_switch.hbs");

const BASH_INTEGRATION_TEMPLATE: &str = include_str!("templates/bash_integration.hbs");

// 其他模板常量...
const POWERSHELL_CC_SWITCH_TEMPLATE: &str = include_str!("templates/powershell_cc_switch.hbs");

const BASH_CC_SWITCH_TEMPLATE: &str = include_str!("templates/bash_cc_switch.hbs");

const FISH_JAVA_SWITCH_TEMPLATE: &str = include_str!("templates/fish_java_switch.hbs");

const FISH_INTEGRATION_TEMPLATE: &str = include_str!("templates/fish_integration.hbs");

const FISH_CC_SWITCH_TEMPLATE: &str = include_str!("templates/fish_cc_switch.hbs");

const CMD_JAVA_SWITCH_TEMPLATE: &str = include_str!("templates/cmd_java_switch.hbs");

const CMD_INTEGRATION_TEMPLATE: &str = include_str!("templates/cmd_integration.hbs");

const CMD_CC_SWITCH_TEMPLATE: &str = include_str!("templates/cmd_cc_switch.hbs");

const POWERSHELL_MAVEN_SWITCH_TEMPLATE: &str =
    include_str!("templates/powershell_maven_switch.hbs");

const BASH_MAVEN_SWITCH_TEMPLATE: &str = include_str!("templates/bash_maven_switch.hbs");

const FISH_MAVEN_SWITCH_TEMPLATE: &str = include_str!("templates/fish_maven_switch.hbs");

const CMD_MAVEN_SWITCH_TEMPLATE: &str = include_str!("templates/cmd_maven_switch.hbs");

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
