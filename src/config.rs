use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// 配置文件结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub java_environments: Vec<JavaEnvironment>,
    #[serde(default)]
    pub llm_environments: Vec<LlmEnvironment>,
    #[serde(default)]
    pub repositories: Repositories,
    /// 当前激活的 Java 环境名称
    #[serde(default)]
    pub current_java_env: Option<String>,
}

/// 仓库配置
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Repositories {
    #[serde(default = "default_java_repositories")]
    pub java: Vec<String>,
    #[serde(default = "default_maven_repositories")]
    pub maven: Vec<String>,
}

fn default_java_repositories() -> Vec<String> {
    vec![
        "https://api.adoptium.net/v3".to_string(),
        "https://api.adoptopenjdk.net/v3".to_string(),
    ]
}

fn default_maven_repositories() -> Vec<String> {
    vec![
        "https://maven.aliyun.com/repository/public".to_string(),
        "https://search.maven.org/solrsearch/select".to_string(),
        "https://repo1.maven.org/maven2".to_string(),
    ]
}

/// Java 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaEnvironment {
    pub name: String,
    pub java_home: String,
    #[serde(default)]
    pub description: String,
}

/// LLM 环境配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmEnvironment {
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub description: String,
}

impl Config {
    /// 创建默认配置
    pub fn new() -> Self {
        Config {
            java_environments: Vec::new(),
            llm_environments: Vec::new(),
            repositories: Repositories {
                java: default_java_repositories(),
                maven: default_maven_repositories(),
            },
            current_java_env: None,
        }
    }

    /// 从文件加载配置
    pub fn load() -> Result<Self, String> {
        let config_path = get_config_path()?;
        
        if !config_path.exists() {
            // 如果配置文件不存在，创建默认配置
            let config = Config::new();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("无法读取配置文件: {}", e))?;
        
        toml::from_str(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))
    }

    /// 保存配置到文件
    pub fn save(&self) -> Result<(), String> {
        let config_path = get_config_path()?;
        
        // 确保配置目录存在
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("无法创建配置目录: {}", e))?;
        }

        let toml_content = toml::to_string_pretty(self)
            .map_err(|e| format!("序列化配置失败: {}", e))?;

        fs::write(&config_path, toml_content)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;

        Ok(())
    }

    /// 添加 Java 环境
    pub fn add_java_env(&mut self, env: JavaEnvironment) -> Result<(), String> {
        // 检查名称是否已存在
        if self.java_environments.iter().any(|e| e.name == env.name) {
            return Err(format!("Java 环境 '{}' 已存在", env.name));
        }
        self.java_environments.push(env);
        Ok(())
    }

    /// 删除 Java 环境
    pub fn remove_java_env(&mut self, name: &str) -> Result<(), String> {
        let original_len = self.java_environments.len();
        self.java_environments.retain(|e| e.name != name);
        if self.java_environments.len() == original_len {
            return Err(format!("Java 环境 '{}' 不存在", name));
        }
        Ok(())
    }

    /// 获取 Java 环境
    pub fn get_java_env(&self, name: &str) -> Option<&JavaEnvironment> {
        self.java_environments.iter().find(|e| e.name == name)
    }

    /// 设置当前激活的 Java 环境
    pub fn set_current_java_env(&mut self, name: String) -> Result<(), String> {
        // 验证环境是否存在
        if !self.java_environments.iter().any(|e| e.name == name) {
            return Err(format!("Java 环境 '{}' 不存在", name));
        }
        self.current_java_env = Some(name);
        Ok(())
    }

    /// 获取当前激活的 Java 环境
    pub fn get_current_java_env(&self) -> Option<&JavaEnvironment> {
        if let Some(ref name) = self.current_java_env {
            self.get_java_env(name)
        } else {
            None
        }
    }

    /// 清除当前激活的 Java 环境
    pub fn clear_current_java_env(&mut self) {
        self.current_java_env = None;
    }

    /// 添加 LLM 环境
    pub fn add_llm_env(&mut self, env: LlmEnvironment) -> Result<(), String> {
        // 检查名称是否已存在
        if self.llm_environments.iter().any(|e| e.name == env.name) {
            return Err(format!("LLM 环境 '{}' 已存在", env.name));
        }
        self.llm_environments.push(env);
        Ok(())
    }

    /// 删除 LLM 环境
    pub fn remove_llm_env(&mut self, name: &str) -> Result<(), String> {
        let original_len = self.llm_environments.len();
        self.llm_environments.retain(|e| e.name != name);
        if self.llm_environments.len() == original_len {
            return Err(format!("LLM 环境 '{}' 不存在", name));
        }
        Ok(())
    }

    /// 获取 LLM 环境
    pub fn get_llm_env(&self, name: &str) -> Option<&LlmEnvironment> {
        self.llm_environments.iter().find(|e| e.name == name)
    }
}

/// 解析环境变量引用（如 ${VAR_NAME}）
pub fn resolve_env_var(value: &str) -> String {
    if value.starts_with("${") && value.ends_with('}') {
        let var_name = &value[2..value.len() - 1];
        env::var(var_name).unwrap_or_else(|_| value.to_string())
    } else {
        value.to_string()
    }
}

/// 获取配置文件路径
pub fn get_config_path() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| "无法获取用户主目录".to_string())?;
    
    let config_file = home_dir.join(".fnva").join("config.toml");
    Ok(config_file)
}

/// 获取配置目录
pub fn get_config_dir() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| "无法获取用户主目录".to_string())?;
    
    Ok(home_dir.join(".fnva"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_env_var() {
        // 设置测试环境变量
        env::set_var("TEST_VAR", "test_value");
        
        let resolved = resolve_env_var("${TEST_VAR}");
        assert_eq!(resolved, "test_value");
        
        let not_resolved = resolve_env_var("normal_value");
        assert_eq!(not_resolved, "normal_value");
        
        env::remove_var("TEST_VAR");
    }

    #[test]
    fn test_config_add_java_env() {
        let mut config = Config::new();
        let env = JavaEnvironment {
            name: "test".to_string(),
            java_home: "/usr/lib/jvm/java-17".to_string(),
            description: "Test JDK".to_string(),
        };
        
        assert!(config.add_java_env(env.clone()).is_ok());
        assert!(config.add_java_env(env).is_err()); // 重复添加应该失败
    }
}
