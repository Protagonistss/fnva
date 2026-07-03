use crate::core::presentation::OutputFormat;

/// 输出格式化器(目前仅用于 use 命令的输出)。
pub struct OutputFormatter;

impl OutputFormatter {
    /// 格式化切换结果
    pub fn format_switch_result(
        &self,
        result: &crate::core::environment_manager::SwitchResult,
        format: OutputFormat,
    ) -> Result<String, String> {
        match format {
            OutputFormat::Text => {
                if result.success {
                    Ok(format!(
                        "Successfully switched to {}: {}\n",
                        result.env_type, result.name
                    ))
                } else {
                    Ok(format!(
                        "Failed to switch to {}: {}\n",
                        result.env_type, result.name
                    ))
                }
            }
            OutputFormat::Json => Ok(serde_json::to_string_pretty(result).unwrap()),
        }
    }
}

/// 默认输出格式化器实例
pub static FORMATTER: OutputFormatter = OutputFormatter;
