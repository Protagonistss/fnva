use std::collections::HashMap;
use std::sync::Arc;

use crate::core::environment_manager::EnvironmentType;
use crate::error::AppError;
use crate::infrastructure::shell::script_strategy::{
    BashStrategy, CmdStrategy, FishStrategy, PowerShellStrategy, ScriptGenerationStrategy,
};
use crate::infrastructure::shell::ShellType;

/// 脚本生成工厂
pub struct ScriptFactory {
    strategies: HashMap<ShellType, Arc<dyn ScriptGenerationStrategy>>,
}

impl ScriptFactory {
    /// 创建新的脚本工厂
    pub fn new() -> Result<Self, AppError> {
        let mut strategies: HashMap<ShellType, Arc<dyn ScriptGenerationStrategy>> = HashMap::new();

        // 注册所有策略
        strategies.insert(ShellType::PowerShell, Arc::new(PowerShellStrategy::new()?));
        strategies.insert(ShellType::Bash, Arc::new(BashStrategy::new()?));
        strategies.insert(ShellType::Zsh, Arc::new(BashStrategy::new()?)); // Zsh使用相同的Bash策略
        strategies.insert(ShellType::Fish, Arc::new(FishStrategy::new()?));
        strategies.insert(ShellType::Cmd, Arc::new(CmdStrategy::new()?));

        Ok(Self { strategies })
    }

    /// 获取指定Shell类型的策略
    pub fn get_strategy(
        &self,
        shell_type: ShellType,
    ) -> Result<Arc<dyn ScriptGenerationStrategy>, AppError> {
        self.strategies
            .get(&shell_type)
            .cloned()
            .ok_or_else(|| AppError::ScriptGeneration {
                shell_type: format!("{:?}", shell_type),
                reason: "不支持的Shell类型".to_string(),
            })
    }

    /// 自动检测Shell并获取策略
    pub fn detect_and_get_strategy(&self) -> Result<Arc<dyn ScriptGenerationStrategy>, AppError> {
        let shell_type = self.detect_shell_type();
        self.get_strategy(shell_type)
    }

    /// 检测当前Shell类型
    pub fn detect_shell_type(&self) -> ShellType {
        // 检查环境变量
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("bash") {
                return ShellType::Bash;
            } else if shell.contains("zsh") {
                return ShellType::Zsh;
            } else if shell.contains("fish") {
                return ShellType::Fish;
            }
        }

        // 检查Windows PowerShell
        if std::env::var("PSModulePath").is_ok() || std::env::var("POSH_THEMES_PATH").is_ok() {
            return ShellType::PowerShell;
        }

        // 检查Windows CMD
        if cfg!(target_os = "windows") && std::env::var("PROMPT").is_ok() {
            return ShellType::Cmd;
        }

        // 默认返回Bash
        ShellType::Bash
    }

    /// 注册自定义策略
    pub fn register_strategy(
        &mut self,
        shell_type: ShellType,
        strategy: Arc<dyn ScriptGenerationStrategy>,
    ) {
        self.strategies.insert(shell_type, strategy);
    }

    /// 获取所有支持的Shell类型
    pub fn supported_shells(&self) -> Vec<ShellType> {
        self.strategies.keys().cloned().collect()
    }

    /// 验证策略是否存在
    pub fn has_strategy(&self, shell_type: ShellType) -> bool {
        self.strategies.contains_key(&shell_type)
    }

    /// 获取策略信息
    pub fn get_strategy_info(&self, shell_type: ShellType) -> Option<StrategyInfo> {
        self.strategies
            .get(&shell_type)
            .map(|strategy| StrategyInfo {
                shell_type: strategy.shell_type(),
                supports_env_vars: strategy.supports_env_vars(),
            })
    }

    /// 获取所有策略信息
    pub fn get_all_strategy_info(&self) -> Vec<StrategyInfo> {
        self.strategies
            .values()
            .map(|strategy| StrategyInfo {
                shell_type: strategy.shell_type(),
                supports_env_vars: strategy.supports_env_vars(),
            })
            .collect()
    }
}

/// 策略信息
#[derive(Debug, Clone)]
pub struct StrategyInfo {
    pub shell_type: ShellType,
    pub supports_env_vars: bool,
}

/// 脚本生成器（简化接口）
pub struct ScriptGenerator {
    factory: ScriptFactory,
}

impl ScriptGenerator {
    /// 创建新的脚本生成器
    pub fn new() -> Result<Self, AppError> {
        Ok(Self {
            factory: ScriptFactory::new()?,
        })
    }

    /// 生成环境切换脚本
    pub fn generate_switch_script(
        &self,
        env_type: EnvironmentType,
        env_name: &str,
        config: &serde_json::Value,
        shell_type: Option<ShellType>,
    ) -> Result<String, AppError> {
        let strategy = if let Some(shell_type) = shell_type {
            self.factory.get_strategy(shell_type)?
        } else {
            self.factory.detect_and_get_strategy()?
        };

        strategy.generate_switch_script(env_type, env_name, config)
    }

    /// 生成集成脚本
    pub fn generate_integration_script(
        &self,
        current_envs: &HashMap<EnvironmentType, String>,
        shell_type: Option<ShellType>,
    ) -> Result<String, AppError> {
        let strategy = if let Some(shell_type) = shell_type {
            self.factory.get_strategy(shell_type)?
        } else {
            self.factory.detect_and_get_strategy()?
        };

        strategy.generate_integration_script(current_envs)
    }

    /// 获取工厂引用
    pub fn factory(&self) -> &ScriptFactory {
        &self.factory
    }
}

impl Default for ScriptGenerator {
    fn default() -> Self {
        Self::new().expect("Failed to create ScriptGenerator")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_script_factory() {
        let factory = ScriptFactory::new().unwrap();

        // 测试获取策略
        let powershell_strategy = factory.get_strategy(ShellType::PowerShell);
        assert!(powershell_strategy.is_ok());

        let unknown_strategy = factory.get_strategy(ShellType::Unknown);
        assert!(unknown_strategy.is_err());

        // 测试支持的Shell类型
        let supported_shells = factory.supported_shells();
        assert!(!supported_shells.is_empty());
        assert!(supported_shells.contains(&ShellType::PowerShell));
        assert!(supported_shells.contains(&ShellType::Bash));
    }

    #[test]
    fn test_shell_detection() {
        let factory = ScriptFactory::new().unwrap();
        let shell_type = factory.detect_shell_type();
        // 测试不会panic
        match shell_type {
            ShellType::PowerShell
            | ShellType::Bash
            | ShellType::Zsh
            | ShellType::Fish
            | ShellType::Cmd
            | ShellType::Unknown => {
                // 都是有效的类型
            }
        }
    }

    #[tokio::test]
    async fn test_script_generator() {
        let generator = ScriptGenerator::new().unwrap();

        let config = json!({
            "java_home": "/test/java"
        });

        // 测试生成脚本（可能因为环境不同而失败）
        let _result = generator
            .generate_switch_script(EnvironmentType::Java, "test", &config, None)
            .await;
    }

    #[test]
    fn test_strategy_info() {
        let factory = ScriptFactory::new().unwrap();

        let info = factory.get_strategy_info(ShellType::PowerShell);
        assert!(info.is_some());
        assert_eq!(info.unwrap().shell_type, ShellType::PowerShell);

        let all_info = factory.get_all_strategy_info();
        assert!(!all_info.is_empty());
        assert!(all_info
            .iter()
            .any(|info| info.shell_type == ShellType::Bash));
    }
}
