use serde::Serialize;

/// 输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

/// 输出格式化器
pub struct OutputFormatter;

impl OutputFormatter {
    /// 格式化环境列表
    pub fn format_environments<T: Serialize + std::fmt::Debug>(
        &self,
        environments: &[T],
        env_type: &str,
        current: Option<&str>,
        format: OutputFormat,
    ) -> Result<String, String> {
        match format {
            OutputFormat::Text => {
                let mut output = String::new();
                if environments.is_empty() {
                    output.push_str(&format!("No {} environments found\n", env_type));
                } else {
                    output.push_str(&format!("Available {} environments:\n", env_type));
                    for env in environments {
                        // 这里需要根据具体类型来格式化
                        // 暂时使用简单的序列化
                        output.push_str(&format!("  {:?}\n", env));
                    }
                    if let Some(current) = current {
                        output.push_str(&format!("Current: {}\n", current));
                    }
                }
                Ok(output)
            }
            OutputFormat::Json => {
                let json_output = serde_json::json!({
                    "environment_type": env_type,
                    "current": current,
                    "environments": environments
                });
                Ok(serde_json::to_string_pretty(&json_output).unwrap())
            }
        }
    }

    /// 格式化切换结果
    pub fn format_switch_result(
        &self,
        result: &crate::core::environment_manager::SwitchResult,
        format: OutputFormat,
    ) -> Result<String, String> {
        match format {
            OutputFormat::Text => {
                if result.success {
                    Ok(format!("Successfully switched to {}: {}\n", result.env_type, result.name))
                } else {
                    Ok(format!("Failed to switch to {}: {}\n", result.env_type, result.name))
                }
            }
            OutputFormat::Json => {
                Ok(serde_json::to_string_pretty(result).unwrap())
            }
        }
    }

    /// 格式化错误信息
    pub fn format_error(&self, error: &str, format: OutputFormat) -> String {
        match format {
            OutputFormat::Text => format!("Error: {}\n", error),
            OutputFormat::Json => {
                let json_output = serde_json::json!({
                    "error": error,
                    "success": false
                });
                serde_json::to_string_pretty(&json_output).unwrap()
            }
        }
    }

    /// 格式化成功信息
    pub fn format_success(&self, message: &str, format: OutputFormat) -> String {
        match format {
            OutputFormat::Text => format!("{}\n", message),
            OutputFormat::Json => {
                let json_output = serde_json::json!({
                    "message": message,
                    "success": true
                });
                serde_json::to_string_pretty(&json_output).unwrap()
            }
        }
    }
}

/// 默认输出格式化器实例
pub static FORMATTER: OutputFormatter = OutputFormatter;