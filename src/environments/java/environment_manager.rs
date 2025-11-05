use std::collections::HashMap;
use crate::core::environment_manager::{EnvironmentManager, EnvironmentType, DynEnvironment, EnvironmentInfo};
use crate::infrastructure::shell::ShellType;
use crate::infrastructure::shell::script_builder::ScriptBuilder;
use crate::environments::java::scanner::JavaScanner;
use serde_json;

/// Java 环境管理器
pub struct JavaEnvironmentManager {
    installations: HashMap<String, crate::environments::java::scanner::JavaInstallation>,
}

impl JavaEnvironmentManager {
    /// 创建新的 Java 环境管理器
    pub fn new() -> Self {
        let mut manager = Self {
            installations: HashMap::new(),
        };

        // 初始化时扫描已有的 Java 环境
        if let Ok(installations) = JavaScanner::scan_system() {
            for installation in installations {
                let name = installation.name.clone();
                manager.installations.insert(name, installation);
            }
        }

        manager
    }
}

impl EnvironmentManager for JavaEnvironmentManager {
    fn environment_type(&self) -> EnvironmentType {
        EnvironmentType::Java
    }

    fn list(&self) -> Result<Vec<DynEnvironment>, String> {
        let mut result = Vec::new();
        for installation in self.installations.values() {
            result.push(DynEnvironment {
                name: installation.name.clone(),
                path: installation.java_home.clone(),
                version: installation.version.clone(),
                description: Some(installation.description.clone()),
                is_active: installation.is_active(),
            });
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
        let config: serde_json::Value = serde_json::from_str(config_str)
            .map_err(|e| format!("Failed to parse config: {}", e))?;

        let java_home = config.get("java_home")
            .and_then(|v| v.as_str())
            .ok_or("Missing java_home in config")?;

        // Validate that it's a valid Java installation
        if !crate::environments::java::scanner::JavaScanner::is_valid_java_installation(java_home) {
            return Err("Invalid Java installation".to_string());
        }

        // Create installation from path
        let installation = crate::environments::java::scanner::JavaScanner::create_installation_from_path(java_home)
            .map_err(|e| format!("Failed to create Java installation: {}", e))?;

        // Override the name with the provided one
        let java_installation = crate::environments::java::scanner::JavaInstallation {
            name: name.to_string(),
            description: format!("Java {} ({})",
                installation.version.as_deref().unwrap_or("unknown"),
                java_home),
            java_home: java_home.to_string(),
            version: installation.version,
            vendor: installation.vendor,
        };

        self.installations.insert(name.to_string(), java_installation);
        Ok(())
    }

    fn remove(&mut self, name: &str) -> Result<(), String> {
        if self.installations.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("Java environment '{}' not found", name))
        }
    }

    fn use_env(&mut self, name: &str, shell_type: Option<ShellType>) -> Result<String, String> {
        let java_installation = self.installations.get(name)
            .ok_or_else(|| format!("Java environment '{}' not found", name))?;

        let shell_type = shell_type.unwrap_or_else(crate::infrastructure::shell::platform::detect_shell);

        let config = serde_json::json!({
            "java_home": java_installation.java_home,
        });

        ScriptBuilder::build_switch_script(
            EnvironmentType::Java,
            name,
            &config,
            shell_type
        )
    }

    fn get_current(&self) -> Result<Option<String>, String> {
        // Check environment variable JAVA_HOME to determine current
        if let Ok(java_home) = std::env::var("JAVA_HOME") {
            // Find which environment matches this JAVA_HOME
            for (name, installation) in &self.installations {
                if installation.java_home == java_home {
                    return Ok(Some(name.clone()));
                }
            }
        }
        Ok(None)
    }

    fn scan(&self) -> Result<Vec<DynEnvironment>, String> {
        let installations = JavaScanner::scan_system()?;
        let mut result = Vec::new();

        for installation in installations {
            result.push(DynEnvironment {
                name: installation.name.clone(),
                path: installation.java_home.clone(),
                version: installation.version.clone(),
                description: Some(installation.description.clone()),
                is_active: installation.is_active(),
            });
        }

        Ok(result)
    }

    fn set_current(&mut self, name: &str) -> Result<(), String> {
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