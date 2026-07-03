use crate::core::environment_manager::{
    DynEnvironment, EnvironmentInfo, EnvironmentManager, EnvironmentType,
};
use crate::core::presentation::ScanHit;
use crate::core::session::SessionManager;
use crate::environments::java::scanner::JavaScanner;
use crate::error::AppError;
use crate::infrastructure::shell::ScriptGenerator;
use crate::infrastructure::shell::ShellType;
use crate::utils::path::normalize_path;
use serde_json;
use std::collections::HashMap;

/// Java 环境管理器
pub struct JavaEnvironmentManager {
    installations: HashMap<String, crate::environments::java::scanner::JavaInstallation>,
}

impl Default for JavaEnvironmentManager {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaEnvironmentManager {
    /// 创建新的 Java 环境管理器（仅从配置文件加载，不进行系统扫描）
    pub fn new() -> Self {
        let mut manager = Self {
            installations: HashMap::new(),
        };

        // 仅从配置文件加载环境
        if let Err(e) = manager.load_from_config() {
            crate::cli::print::warn(&format!("Failed to load environments from config: {e}"));
        }

        manager
    }

    /// 从配置文件加载 Java 环境
    fn load_from_config(&mut self) -> Result<(), AppError> {
        use crate::infrastructure::config::Config;

        let config = Config::load().map_err(|e| AppError::config_error(&e))?;

        // 清除旧的环境数据，确保重新加载最新的配置
        self.installations.clear();

        for env in &config.java_environments {
            // 移除了黑名单检查逻辑，现在允许所有环境重新加载
            // 原来：if config.is_java_name_removed(&env.name) { continue; }

            let installation = crate::environments::java::scanner::JavaInstallation {
                name: env.name.clone(),
                description: env.description.clone(),
                java_home: env.java_home.clone(),
                version: None, // 将在需要时检测
                vendor: None,  // 将在需要时检测
            };

            self.installations.insert(env.name.clone(), installation);
        }

        Ok(())
    }

    /// 保存环境到配置文件
    fn save_to_config_impl(name: &str, java_home: &str, description: &str) -> Result<(), AppError> {
        use crate::infrastructure::config::{Config, JavaEnvironment};

        let mut config = Config::load().map_err(|e| AppError::config_error(&e))?;

        // Check if environment already exists and update it (overwrite)
        if let Some(existing_env) = config
            .java_environments
            .iter_mut()
            .find(|env| env.name == name)
        {
            // Update existing environment
            existing_env.java_home = java_home.to_string();
            existing_env.description = description.to_string();
            existing_env.source = crate::infrastructure::config::EnvironmentSource::Manual;
        } else {
            // Add new environment
            let new_env = JavaEnvironment {
                name: name.to_string(),
                java_home: java_home.to_string(),
                description: description.to_string(),
                source: crate::infrastructure::config::EnvironmentSource::Manual,
            };
            config.java_environments.push(new_env);
        }

        config.save().map_err(|e| AppError::config_error(&e))?;

        Ok(())
    }

    /// 从移除列表中移除名称（允许重新添加）
    fn remove_name_from_removed_list(name: &str) -> Result<(), AppError> {
        use crate::infrastructure::config::Config;

        let mut config = Config::load().map_err(|e| AppError::config_error(&e))?;
        config.remove_java_name_from_removed_list(name);
        config.save().map_err(|e| AppError::config_error(&e))?;
        Ok(())
    }

    /// 从配置文件中删除环境
    fn remove_from_config(name: &str) -> Result<(), AppError> {
        use crate::infrastructure::config::Config;

        let mut config = Config::load().map_err(|e| AppError::config_error(&e))?;

        // 查找并删除指定的环境
        let original_len = config.java_environments.len();
        config.java_environments.retain(|env| env.name != name);

        if config.java_environments.len() == original_len {
            return Err(AppError::not_found(&format!(
                "Java environment '{name}' not found in config"
            )));
        }

        // 如果删除的是默认环境，清理默认环境设置
        if config
            .default_java_env
            .as_ref()
            .is_some_and(|default| default == name)
        {
            config.default_java_env = None;
        }

        // 修复：不将删除的环境名加入黑名单，允许用户重新安装相同名字的环境
        // 移除了：config.add_removed_java_name(name);

        // 保存配置文件
        config.save().map_err(|e| AppError::config_error(&e))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EnvironmentManager for JavaEnvironmentManager {
    fn environment_type(&self) -> EnvironmentType {
        EnvironmentType::Java
    }

    fn list(&self) -> Result<Vec<DynEnvironment>, AppError> {
        let mut result = Vec::new();
        let current_env = self.get_current().ok().flatten();

        for (name, env) in &self.installations {
            let is_active = current_env.as_ref() == Some(name);
            let environment = DynEnvironment {
                name: env.name.clone(),
                path: env.java_home.clone(),
                version: None, // 版本信息在需要时动态检测
                description: Some(env.description.clone()),
                is_active,
            };

            result.push(environment);
        }

        Ok(result)
    }

    fn get(&self, name: &str) -> Result<Option<DynEnvironment>, AppError> {
        if let Some(installation) = self.installations.get(name) {
            Ok(Some(DynEnvironment {
                name: installation.name.clone(),
                path: installation.java_home.clone(),
                version: installation.version.clone(),
                description: Some(installation.description.clone()),
                is_active: installation.is_active(),
            }))
        } else {
            Ok(None)
        }
    }

    fn add(&mut self, name: &str, config_str: &str) -> Result<(), AppError> {
        // Parse config as JSON to extract java_home
        let config: serde_json::Value = serde_json::from_str(config_str)?;

        let java_home = config
            .get("java_home")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::validation("java_home", "missing in config"))?;

        // Validate that it's a valid Java installation
        if !crate::environments::java::scanner::JavaScanner::is_valid_java_installation(java_home) {
            return Err(AppError::validation(
                "java_home",
                "Invalid Java installation",
            ));
        }

        // Create installation from path
        let installation =
            crate::environments::java::scanner::JavaScanner::create_installation_from_path(
                java_home,
            )
            .map_err(|e| {
                AppError::validation(
                    "java_home",
                    &format!("Failed to create Java installation: {e}"),
                )
            })?;

        // Extract version info before moving
        let version_info = installation.version.as_deref().unwrap_or("unknown");

        // Override the name with the provided one
        let java_installation = crate::environments::java::scanner::JavaInstallation {
            name: name.to_string(),
            description: format!("Java {version_info} ({java_home})"),
            java_home: java_home.to_string(),
            version: installation.version.clone(),
            vendor: installation.vendor,
        };

        // If this name was previously removed, remove it from the removed list
        Self::remove_name_from_removed_list(name)?;

        // Add to in-memory installations
        self.installations
            .insert(name.to_string(), java_installation);

        // Also save to configuration file
        Self::save_to_config_impl(
            name,
            java_home,
            &format!("Java {version_info} ({java_home})"),
        )?;

        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), AppError> {
        // 首先从内存中移除
        if self.installations.remove(name).is_some() {
            // 尝试从配置文件中删除（如果存在的话）
            if let Err(e) = Self::remove_from_config(name) {
                // 如果配置文件中没有这个环境，那也没关系
                // 可能是通过扫描发现的环境
                crate::cli::print::warn(&e.to_string());
            }
            Ok(())
        } else {
            Err(AppError::not_found(&format!("Java environment '{name}'")))
        }
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, AppError> {
        let java_installation = self
            .installations
            .get(name)
            .ok_or_else(|| AppError::not_found(&format!("Java environment '{name}'")))?;

        // 验证 java_home 路径是否真实存在且包含有效的 Java 安装
        if !crate::utils::validate_java_home(&java_installation.java_home) {
            let java_home = &java_installation.java_home;
            let path_exists = std::path::Path::new(java_home).exists();
            let reason = if path_exists {
                format!("Java installation at '{java_home}' is incomplete or corrupted")
            } else {
                format!("Java installation path does not exist: {java_home}")
            };
            return Err(AppError::validation("java_home", &reason));
        }

        let shell_type =
            shell_type.unwrap_or_else(crate::infrastructure::shell::platform::detect_shell);

        let config = serde_json::json!({
            "java_home": java_installation.java_home,
        });

        let generator = ScriptGenerator::new()?;
        generator.generate_switch_script(EnvironmentType::Java, name, &config, Some(shell_type))
    }

    fn get_current(&self) -> Result<Option<String>, AppError> {
        // Session 优先
        if let Ok(session) = SessionManager::new() {
            if let Some(current) = session.get_current_environment(EnvironmentType::Java) {
                return Ok(Some(current.clone()));
            }
        }

        // Check environment variable JAVA_HOME to determine current
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            // Normalize the JAVA_HOME path for comparison
            let normalized_current = normalize_path(&java_home);

            // Find which environment matches this JAVA_HOME
            for (name, installation) in &self.installations {
                let normalized_installation = normalize_path(&installation.java_home);
                if normalized_installation == normalized_current {
                    return Ok(Some(name.clone()));
                }
            }
        }
        Ok(None)
    }

    async fn scan(&self, extra: &[String]) -> Result<Vec<ScanHit>, AppError> {
        let mut hits = JavaScanner::scan_system(extra)
            .await
            .map_err(|e| AppError::from_string(&e))?;
        // 去重已配置的环境(按归一化路径)
        let configured: std::collections::HashSet<String> = self
            .installations
            .values()
            .map(|i| normalize_path(&i.java_home))
            .collect();
        hits.retain(|h| !configured.contains(&normalize_path(&h.location)));
        Ok(hits)
    }

    fn set_current(&mut self, _name: &str) -> Result<(), AppError> {
        // This would set the current environment, but for Java this is typically
        // handled by setting JAVA_HOME environment variable
        // For now, this is a no-op
        Ok(())
    }

    fn is_available(&self, name: &str) -> Result<bool, AppError> {
        Ok(self.installations.contains_key(name))
    }

    fn get_details(&self, name: &str) -> Result<Option<DynEnvironment>, AppError> {
        self.get(name)
    }
}
