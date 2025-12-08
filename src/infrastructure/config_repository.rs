use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::infrastructure::config::{CcEnvironment, JavaEnvironment, LlmEnvironment};

/// 配置仓储抽象接口
#[async_trait]
pub trait ConfigRepository: Send + Sync {
    /// 加载Java环境配置
    async fn load_java_environments(&self) -> Result<Vec<JavaEnvironment>, AppError>;

    /// 保存Java环境配置
    async fn save_java_environments(
        &self,
        environments: &[JavaEnvironment],
    ) -> Result<(), AppError>;

    /// 加载LLM环境配置
    async fn load_llm_environments(&self) -> Result<Vec<LlmEnvironment>, AppError>;

    /// 保存LLM环境配置
    async fn save_llm_environments(&self, environments: &[LlmEnvironment]) -> Result<(), AppError>;

    /// 加载CC环境配置
    async fn load_cc_environments(&self) -> Result<Vec<CcEnvironment>, AppError>;

    /// 保存CC环境配置
    async fn save_cc_environments(&self, environments: &[CcEnvironment]) -> Result<(), AppError>;

    /// 加载全局设置
    async fn load_global_settings(&self) -> Result<GlobalSettings, AppError>;

    /// 保存全局设置
    async fn save_global_settings(&self, settings: &GlobalSettings) -> Result<(), AppError>;
}

/// 全局设置配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalSettings {
    /// 当前激活的Java环境名称
    pub current_java_env: Option<String>,
    /// 默认Java环境名称
    pub default_java_env: Option<String>,
    /// 默认CC环境名称
    pub default_cc_env: Option<String>,
    /// 自定义Java扫描路径
    pub custom_java_scan_paths: Vec<String>,
    /// 已移除的Java环境名称列表
    pub removed_java_names: Vec<String>,
}

/// 基于文件的配置仓储实现
#[allow(dead_code)] // 兼容保留：当前未用到缓存，避免编译警告
pub struct FileSystemConfigRepository {
    config_dir: PathBuf,
    cache: Arc<RwLock<HashMap<String, String>>>,
}

#[allow(dead_code)] // 兼容保留未使用的方法
impl FileSystemConfigRepository {
    /// 创建新的文件系统配置仓储
    pub fn new(config_dir: PathBuf) -> Result<Self, AppError> {
        // 确保配置目录存在
        std::fs::create_dir_all(&config_dir).map_err(|e| AppError::Io(e.to_string()))?;

        Ok(Self {
            config_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// 获取Java配置文件路径
    fn java_config_path(&self) -> PathBuf {
        self.config_dir.join("java.toml")
    }

    /// 获取LLM配置文件路径
    fn llm_config_path(&self) -> PathBuf {
        self.config_dir.join("llm.toml")
    }

    /// 获取CC配置文件路径
    fn cc_config_path(&self) -> PathBuf {
        self.config_dir.join("cc.toml")
    }

    /// 获取全局设置文件路径
    fn global_settings_path(&self) -> PathBuf {
        self.config_dir.join("global.toml")
    }

    /// 读取TOML文件
    async fn read_toml_file<T>(&self, path: &Path) -> Result<T, AppError>
    where
        T: for<'de> Deserialize<'de> + Default,
    {
        if !path.exists() {
            return Ok(T::default());
        }

        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;

        toml::from_str(&content).map_err(|e| AppError::Serialization(e.to_string()))
    }

    /// 写入TOML文件
    async fn write_toml_file<T>(&self, path: &Path, data: &T) -> Result<(), AppError>
    where
        T: Serialize,
    {
        let content =
            toml::to_string_pretty(data).map_err(|e| AppError::Serialization(e.to_string()))?;

        tokio::fs::write(path, content)
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;

        Ok(())
    }

    /// 获取文件修改时间用于缓存
    async fn get_file_modified(
        &self,
        path: &Path,
    ) -> Result<Option<std::time::SystemTime>, AppError> {
        if !path.exists() {
            return Ok(None);
        }

        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| AppError::Io(e.to_string()))?;

        Ok(Some(
            metadata
                .modified()
                .map_err(|e| AppError::Io(e.to_string()))?,
        ))
    }

    /// 检查缓存是否有效
    async fn is_cache_valid(&self, _path: &Path) -> bool {
        // 简化实现：暂时禁用缓存，后续可以基于文件修改时间实现
        false
    }
}

#[async_trait]
impl ConfigRepository for FileSystemConfigRepository {
    async fn load_java_environments(&self) -> Result<Vec<JavaEnvironment>, AppError> {
        let path = self.java_config_path();

        #[derive(Debug, Deserialize, Default)]
        struct JavaConfig {
            #[serde(default)]
            environments: Vec<JavaEnvironment>,
        }

        let config: JavaConfig = self.read_toml_file(&path).await?;
        Ok(config.environments)
    }

    async fn save_java_environments(
        &self,
        environments: &[JavaEnvironment],
    ) -> Result<(), AppError> {
        let path = self.java_config_path();

        #[derive(Debug, Serialize)]
        struct JavaConfig {
            environments: Vec<JavaEnvironment>,
        }

        let config = JavaConfig {
            environments: environments.to_vec(),
        };

        self.write_toml_file(&path, &config).await
    }

    async fn load_llm_environments(&self) -> Result<Vec<LlmEnvironment>, AppError> {
        let path = self.llm_config_path();

        #[derive(Debug, Deserialize, Default)]
        struct LlmConfig {
            #[serde(default)]
            environments: Vec<LlmEnvironment>,
        }

        let config: LlmConfig = self.read_toml_file(&path).await?;
        Ok(config.environments)
    }

    async fn save_llm_environments(&self, environments: &[LlmEnvironment]) -> Result<(), AppError> {
        let path = self.llm_config_path();

        #[derive(Debug, Serialize)]
        struct LlmConfig {
            environments: Vec<LlmEnvironment>,
        }

        let config = LlmConfig {
            environments: environments.to_vec(),
        };

        self.write_toml_file(&path, &config).await
    }

    async fn load_cc_environments(&self) -> Result<Vec<CcEnvironment>, AppError> {
        let path = self.cc_config_path();

        #[derive(Debug, Deserialize, Default)]
        struct CcConfig {
            #[serde(default)]
            environments: Vec<CcEnvironment>,
        }

        let config: CcConfig = self.read_toml_file(&path).await?;
        Ok(config.environments)
    }

    async fn save_cc_environments(&self, environments: &[CcEnvironment]) -> Result<(), AppError> {
        let path = self.cc_config_path();

        #[derive(Debug, Serialize)]
        struct CcConfig {
            environments: Vec<CcEnvironment>,
        }

        let config = CcConfig {
            environments: environments.to_vec(),
        };

        self.write_toml_file(&path, &config).await
    }

    async fn load_global_settings(&self) -> Result<GlobalSettings, AppError> {
        let path = self.global_settings_path();
        self.read_toml_file(&path).await
    }

    async fn save_global_settings(&self, settings: &GlobalSettings) -> Result<(), AppError> {
        let path = self.global_settings_path();
        self.write_toml_file(&path, settings).await
    }
}

/// 配置管理器 - 提供高级配置操作
pub struct ConfigManager {
    repository: Arc<dyn ConfigRepository>,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(repository: Arc<dyn ConfigRepository>) -> Self {
        Self { repository }
    }

    /// 创建基于文件系统的配置管理器
    pub async fn new_file_system(config_dir: PathBuf) -> Result<Self, AppError> {
        let repository = Arc::new(FileSystemConfigRepository::new(config_dir)?);
        Ok(Self::new(repository))
    }

    /// 添加Java环境
    pub async fn add_java_environment(&self, env: JavaEnvironment) -> Result<(), AppError> {
        let mut environments = self.repository.load_java_environments().await?;

        // 检查名称是否已存在
        if environments.iter().any(|e| e.name == env.name) {
            return Err(AppError::Environment {
                message: format!("Java环境 '{}' 已存在", env.name),
            });
        }

        environments.push(env);
        self.repository.save_java_environments(&environments).await
    }

    /// 删除Java环境
    pub async fn remove_java_environment(&self, name: &str) -> Result<(), AppError> {
        let mut environments = self.repository.load_java_environments().await?;
        let original_len = environments.len();

        environments.retain(|e| e.name != name);

        if environments.len() == original_len {
            return Err(AppError::Environment {
                message: format!("Java环境 '{name}' 不存在"),
            });
        }

        self.repository.save_java_environments(&environments).await
    }

    /// 获取Java环境
    pub async fn get_java_environment(
        &self,
        name: &str,
    ) -> Result<Option<JavaEnvironment>, AppError> {
        let environments = self.repository.load_java_environments().await?;
        Ok(environments.into_iter().find(|e| e.name == name))
    }

    /// 列出所有Java环境
    pub async fn list_java_environments(&self) -> Result<Vec<JavaEnvironment>, AppError> {
        self.repository.load_java_environments().await
    }

    /// 添加LLM环境
    pub async fn add_llm_environment(&self, env: LlmEnvironment) -> Result<(), AppError> {
        let mut environments = self.repository.load_llm_environments().await?;

        // 检查名称是否已存在
        if environments.iter().any(|e| e.name == env.name) {
            return Err(AppError::Environment {
                message: format!("LLM环境 '{}' 已存在", env.name),
            });
        }

        environments.push(env);
        self.repository.save_llm_environments(&environments).await
    }

    /// 删除LLM环境
    pub async fn remove_llm_environment(&self, name: &str) -> Result<(), AppError> {
        let mut environments = self.repository.load_llm_environments().await?;
        let original_len = environments.len();

        environments.retain(|e| e.name != name);

        if environments.len() == original_len {
            return Err(AppError::Environment {
                message: format!("LLM环境 '{name}' 不存在"),
            });
        }

        self.repository.save_llm_environments(&environments).await
    }

    /// 获取LLM环境
    pub async fn get_llm_environment(
        &self,
        name: &str,
    ) -> Result<Option<LlmEnvironment>, AppError> {
        let environments = self.repository.load_llm_environments().await?;
        Ok(environments.into_iter().find(|e| e.name == name))
    }

    /// 列出所有LLM环境
    pub async fn list_llm_environments(&self) -> Result<Vec<LlmEnvironment>, AppError> {
        self.repository.load_llm_environments().await
    }

    /// 添加CC环境
    pub async fn add_cc_environment(&self, env: CcEnvironment) -> Result<(), AppError> {
        let mut environments = self.repository.load_cc_environments().await?;

        // 检查名称是否已存在
        if environments.iter().any(|e| e.name == env.name) {
            return Err(AppError::Environment {
                message: format!("CC环境 '{}' 已存在", env.name),
            });
        }

        environments.push(env);
        self.repository.save_cc_environments(&environments).await
    }

    /// 删除CC环境
    pub async fn remove_cc_environment(&self, name: &str) -> Result<(), AppError> {
        let mut environments = self.repository.load_cc_environments().await?;
        let original_len = environments.len();

        environments.retain(|e| e.name != name);

        if environments.len() == original_len {
            return Err(AppError::Environment {
                message: format!("CC环境 '{name}' 不存在"),
            });
        }

        self.repository.save_cc_environments(&environments).await
    }

    /// 获取CC环境
    pub async fn get_cc_environment(&self, name: &str) -> Result<Option<CcEnvironment>, AppError> {
        let environments = self.repository.load_cc_environments().await?;
        Ok(environments.into_iter().find(|e| e.name == name))
    }

    /// 列出所有CC环境
    pub async fn list_cc_environments(&self) -> Result<Vec<CcEnvironment>, AppError> {
        self.repository.load_cc_environments().await
    }

    /// 获取全局设置
    pub async fn get_global_settings(&self) -> Result<GlobalSettings, AppError> {
        self.repository.load_global_settings().await
    }

    /// 更新全局设置
    pub async fn update_global_settings<F>(&self, updater: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut GlobalSettings),
    {
        let mut settings = self.repository.load_global_settings().await?;
        updater(&mut settings);
        self.repository.save_global_settings(&settings).await
    }

    /// 设置当前Java环境
    pub async fn set_current_java_env(&self, name: &str) -> Result<(), AppError> {
        // 验证环境是否存在
        if self.get_java_environment(name).await?.is_none() {
            return Err(AppError::Environment {
                message: format!("Java环境 '{name}' 不存在"),
            });
        }

        self.update_global_settings(|settings| {
            settings.current_java_env = Some(name.to_string());
        })
        .await
    }

    /// 获取当前Java环境
    pub async fn get_current_java_env(&self) -> Result<Option<JavaEnvironment>, AppError> {
        let settings = self.repository.load_global_settings().await?;
        if let Some(ref name) = settings.current_java_env {
            self.get_java_environment(name).await
        } else {
            Ok(None)
        }
    }

    /// 设置默认Java环境
    pub async fn set_default_java_env(&self, name: &str) -> Result<(), AppError> {
        self.update_global_settings(|settings| {
            settings.default_java_env = Some(name.to_string());
        })
        .await
    }

    /// 获取默认Java环境
    pub async fn get_default_java_env(&self) -> Result<Option<JavaEnvironment>, AppError> {
        let settings = self.repository.load_global_settings().await?;
        if let Some(ref name) = settings.default_java_env {
            self.get_java_environment(name).await
        } else {
            Ok(None)
        }
    }

    /// 获取有效的Java环境（当前 -> 默认）
    pub async fn get_effective_java_env(&self) -> Result<Option<JavaEnvironment>, AppError> {
        // 尝试获取当前环境
        if let Some(env) = self.get_current_java_env().await? {
            return Ok(Some(env));
        }

        // 尝试获取默认环境
        self.get_default_java_env().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::config::EnvironmentSource;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_java_environments() {
        let temp_dir = TempDir::new().unwrap();
        let config_manager = ConfigManager::new_file_system(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // 添加Java环境
        let env = JavaEnvironment {
            name: "test-jdk".to_string(),
            java_home: "/usr/lib/jvm/java-17".to_string(),
            description: "Test JDK".to_string(),
            source: EnvironmentSource::Manual,
        };

        config_manager
            .add_java_environment(env.clone())
            .await
            .unwrap();

        // 列出环境
        let environments = config_manager.list_java_environments().await.unwrap();
        assert_eq!(environments.len(), 1);
        assert_eq!(environments[0].name, "test-jdk");

        // 获取环境
        let retrieved_env = config_manager
            .get_java_environment("test-jdk")
            .await
            .unwrap();
        assert!(retrieved_env.is_some());
        assert_eq!(retrieved_env.unwrap().java_home, "/usr/lib/jvm/java-17");

        // 删除环境
        config_manager
            .remove_java_environment("test-jdk")
            .await
            .unwrap();
        let environments = config_manager.list_java_environments().await.unwrap();
        assert_eq!(environments.len(), 0);
    }

    #[tokio::test]
    async fn test_config_manager_global_settings() {
        let temp_dir = TempDir::new().unwrap();
        let config_manager = ConfigManager::new_file_system(temp_dir.path().to_path_buf())
            .await
            .unwrap();

        // 添加Java环境
        let env = JavaEnvironment {
            name: "test-jdk".to_string(),
            java_home: "/usr/lib/jvm/java-17".to_string(),
            description: "Test JDK".to_string(),
            source: EnvironmentSource::Manual,
        };
        config_manager.add_java_environment(env).await.unwrap();

        // 设置当前环境
        config_manager
            .set_current_java_env("test-jdk")
            .await
            .unwrap();

        // 获取当前环境
        let current_env = config_manager.get_current_java_env().await.unwrap();
        assert!(current_env.is_some());
        assert_eq!(current_env.unwrap().name, "test-jdk");

        // 设置默认环境
        config_manager
            .set_default_java_env("test-jdk")
            .await
            .unwrap();

        // 获取有效环境
        let effective_env = config_manager.get_effective_java_env().await.unwrap();
        assert!(effective_env.is_some());
        assert_eq!(effective_env.unwrap().name, "test-jdk");
    }
}
