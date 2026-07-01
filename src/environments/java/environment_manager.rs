use crate::core::environment_manager::{
    DynEnvironment, EnvironmentInfo, EnvironmentManager, EnvironmentType,
};
use crate::core::session::SessionManager;
use crate::environments::java::scanner::JavaScanner;
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

    /// 创建新的 Java 环境管理器并进行系统扫描
    pub async fn new_with_scan() -> Self {
        let mut manager = Self::new();

        // 扫描系统中的 Java 环境，添加新发现的环境
        if let Ok(installations) = JavaScanner::scan_system().await {
            for installation in installations {
                let name = installation.name.clone();
                // 只有当环境中不存在时才添加
                manager.installations.entry(name).or_insert_with(|| {
                    // 将扫描发现的环境也保存到配置文件中
                    if let Err(e) = Self::save_scanned_environment_to_config(&installation) {
                        crate::cli::print::warn(&format!(
                            "Failed to save scanned environment to config: {e}"
                        ));
                    }
                    installation
                });
            }
        }

        manager
    }

    /// 扫描系统并更新环境列表
    pub async fn scan_and_update(&mut self) -> Result<(), String> {
        // 扫描系统中的 Java 环境
        let installations = JavaScanner::scan_system().await?;

        for installation in installations {
            let name = installation.name.clone();
            // 只有当环境中不存在时才添加
            self.installations.entry(name).or_insert_with(|| {
                // 将扫描发现的环境保存到配置文件中
                if let Err(e) = Self::save_scanned_environment_to_config(&installation) {
                    crate::cli::print::warn(&format!(
                        "Failed to save scanned environment to config: {e}"
                    ));
                }
                installation
            });
        }

        Ok(())
    }

    /// 从配置文件加载 Java 环境
    fn load_from_config(&mut self) -> Result<(), String> {
        use crate::infrastructure::config::Config;

        let config = Config::load()?;

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
    fn save_to_config_impl(name: &str, java_home: &str, description: &str) -> Result<(), String> {
        use crate::infrastructure::config::{Config, JavaEnvironment};

        let mut config = Config::load()?;

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

        config.save()?;

        Ok(())
    }

    /// 将扫描发现的环境保存到配置文件
    fn save_scanned_environment_to_config(
        installation: &crate::environments::java::scanner::JavaInstallation,
    ) -> Result<(), String> {
        use crate::infrastructure::config::{Config, EnvironmentSource, JavaEnvironment};

        let mut config = Config::load()?;

        // 检查是否已经存在，如果存在则更新（覆盖）
        if let Some(existing_env) = config
            .java_environments
            .iter_mut()
            .find(|env| env.name == installation.name)
        {
            // 更新现有环境的信息
            existing_env.java_home = installation.java_home.clone();
            existing_env.description = installation.description.clone();
            if existing_env.source == EnvironmentSource::Manual {
                // 如果是手动添加的，保持 source 为 Manual
            } else {
                existing_env.source = EnvironmentSource::Scanned;
            }
            config.save()?;
            return Ok(());
        }

        // 添加新的扫描发现的环境
        let scanned_env = JavaEnvironment {
            name: installation.name.clone(),
            java_home: installation.java_home.clone(),
            description: installation.description.clone(),
            source: EnvironmentSource::Scanned,
        };

        config.java_environments.push(scanned_env);
        config.save()?;

        Ok(())
    }

    /// 从移除列表中移除名称（允许重新添加）
    fn remove_name_from_removed_list(name: &str) -> Result<(), String> {
        use crate::infrastructure::config::Config;

        let mut config = Config::load()?;
        config.remove_java_name_from_removed_list(name);
        config.save()?;
        Ok(())
    }

    /// 从配置文件中删除环境
    fn remove_from_config(name: &str) -> Result<(), String> {
        use crate::infrastructure::config::Config;

        let mut config = Config::load()?;

        // 查找并删除指定的环境
        let original_len = config.java_environments.len();
        config.java_environments.retain(|env| env.name != name);

        if config.java_environments.len() == original_len {
            return Err(format!("Java environment '{name}' not found in config"));
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
        config.save()?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl EnvironmentManager for JavaEnvironmentManager {
    fn environment_type(&self) -> EnvironmentType {
        EnvironmentType::Java
    }

    fn list(&self) -> Result<Vec<DynEnvironment>, String> {
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

    fn get(&self, name: &str) -> Result<Option<DynEnvironment>, String> {
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

    fn add(&mut self, name: &str, config_str: &str) -> Result<(), String> {
        // Parse config as JSON to extract java_home
        let config: serde_json::Value =
            serde_json::from_str(config_str).map_err(|e| format!("Failed to parse config: {e}"))?;

        let java_home = config
            .get("java_home")
            .and_then(|v| v.as_str())
            .ok_or("Missing java_home in config")?;

        // Validate that it's a valid Java installation
        if !crate::environments::java::scanner::JavaScanner::is_valid_java_installation(java_home) {
            return Err("Invalid Java installation".to_string());
        }

        // Create installation from path
        let installation =
            crate::environments::java::scanner::JavaScanner::create_installation_from_path(
                java_home,
            )
            .map_err(|e| format!("Failed to create Java installation: {e}"))?;

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

    fn remove(&mut self, name: &str) -> Result<(), String> {
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
            Err(format!("Java environment '{name}' not found"))
        }
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, String> {
        let java_installation = self
            .installations
            .get(name)
            .ok_or_else(|| {
                let mut msg = format!("Java environment '{name}' not found.");
                let pkg_dir = crate::infrastructure::paths::tool_packages_dir("java").unwrap_or_default();
                if std::path::Path::new(&pkg_dir).join(name).exists() {
                    let pkg_dir_str = pkg_dir.to_string_lossy();
                    msg.push_str(&format!("\n  Hint: Java files exist at {pkg_dir_str}/{name}, but no config entry found."));
                    msg.push_str("\n  Try: fnva java scan  or  fnva java install <version>");
                } else {
                    msg.push_str("\n  Available environments: fnva java list");
                    msg.push_str("\n  Install a new one: fnva java install <version>");
                }
                msg
            })?;

        // 验证 java_home 路径是否真实存在且包含有效的 Java 安装
        if !crate::utils::validate_java_home(&java_installation.java_home) {
            let java_home = &java_installation.java_home;
            let path_exists = std::path::Path::new(java_home).exists();

            let mut msg = if path_exists {
                format!("Java installation at '{java_home}' is incomplete or corrupted.")
            } else {
                format!("Java installation path does not exist: {java_home}")
            };
            msg.push_str("\n  The configured path is invalid. Possible causes:");
            msg.push_str("\n    - Installation was moved or deleted");
            msg.push_str("\n    - Download was incomplete");
            msg.push_str("\n  Fix: fnva java install <version> to reinstall");

            return Err(msg);
        }

        let shell_type =
            shell_type.unwrap_or_else(crate::infrastructure::shell::platform::detect_shell);

        let config = serde_json::json!({
            "java_home": java_installation.java_home,
        });

        let generator = ScriptGenerator::new().map_err(|e| e.to_string())?;
        match generator.generate_switch_script(
            EnvironmentType::Java,
            name,
            &config,
            Some(shell_type),
        ) {
            Ok(script) => Ok(script),
            Err(e) => Err(format!("Failed to generate script: {e}")),
        }
    }

    fn get_current(&self) -> Result<Option<String>, String> {
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

    async fn scan(&self) -> Result<Vec<DynEnvironment>, String> {
        let installations = JavaScanner::scan_system().await?;
        let mut result = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        let current_env = self.get_current().ok().flatten();

        // 首先添加已配置的环境（优先级更高）
        for (name, installation) in &self.installations {
            let normalized_path = normalize_path(&installation.java_home);
            if !seen_paths.contains(&normalized_path) {
                // 检测版本信息（如果还没有）
                let version = if installation.version.is_none() {
                    JavaScanner::detect_java_version(&installation.java_home)
                        .ok()
                        .flatten()
                } else {
                    installation.version.clone()
                };

                let is_active = current_env.as_ref() == Some(name);

                result.push(DynEnvironment {
                    name: name.clone(),
                    path: installation.java_home.clone(),
                    version,
                    description: Some(installation.description.clone()),
                    is_active,
                });
                seen_paths.insert(normalized_path);
            }
        }

        // 添加扫描到的环境
        for installation in installations {
            let normalized_path = normalize_path(&installation.java_home);
            if !seen_paths.contains(&normalized_path) {
                let is_active = current_env.as_ref() == Some(&installation.name);
                result.push(DynEnvironment {
                    name: installation.name.clone(),
                    path: installation.java_home.clone(),
                    version: installation.version.clone(),
                    description: Some(installation.description.clone()),
                    is_active,
                });
                seen_paths.insert(normalized_path);
            }
        }

        // 按名称排序
        result.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(result)
    }

    fn set_current(&mut self, _name: &str) -> Result<(), String> {
        // This would set the current environment, but for Java this is typically
        // handled by setting JAVA_HOME environment variable
        // For now, this is a no-op
        Ok(())
    }

    fn is_available(&self, name: &str) -> Result<bool, String> {
        Ok(self.installations.contains_key(name))
    }

    fn get_details(&self, name: &str) -> Result<Option<DynEnvironment>, String> {
        self.get(name)
    }
}
