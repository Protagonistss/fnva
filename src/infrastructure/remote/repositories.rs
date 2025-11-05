use serde::{Deserialize, Serialize};

/// 仓库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    /// 仓库名称
    pub name: String,
    /// 仓库 URL
    pub url: String,
    /// 仓库类型
    pub repo_type: RepositoryType,
    /// 是否启用
    pub enabled: bool,
    /// 优先级
    pub priority: i32,
}

/// 仓库类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum RepositoryType {
    Java,
    Maven,
    Npm,
    Docker,
}

/// 仓库管理器
pub struct RepositoryManager;

impl RepositoryManager {
    /// 获取默认的 Java 仓库
    pub fn get_default_java_repositories() -> Vec<RepositoryConfig> {
        vec![
            RepositoryConfig {
                name: "Adoptium".to_string(),
                url: "https://api.adoptium.net/v3".to_string(),
                repo_type: RepositoryType::Java,
                enabled: true,
                priority: 1,
            },
            RepositoryConfig {
                name: "Aliyun Mirror".to_string(),
                url: "https://mirrors.aliyun.com/adoptium".to_string(),
                repo_type: RepositoryType::Java,
                enabled: true,
                priority: 2,
            },
        ]
    }

    /// 获取默认的 Maven 仓库
    pub fn get_default_maven_repositories() -> Vec<RepositoryConfig> {
        vec![
            RepositoryConfig {
                name: "Maven Central".to_string(),
                url: "https://search.maven.org/solrsearch/select".to_string(),
                repo_type: RepositoryType::Maven,
                enabled: true,
                priority: 1,
            },
            RepositoryConfig {
                name: "Aliyun Maven".to_string(),
                url: "https://maven.aliyun.com/repository/public".to_string(),
                repo_type: RepositoryType::Maven,
                enabled: true,
                priority: 2,
            },
        ]
    }

    /// 根据类型获取仓库
    pub fn get_repositories_by_type(repo_type: RepositoryType) -> Vec<RepositoryConfig> {
        match repo_type {
            RepositoryType::Java => Self::get_default_java_repositories(),
            RepositoryType::Maven => Self::get_default_maven_repositories(),
            _ => Vec::new(),
        }
    }

    /// 根据名称查找仓库
    pub fn find_repository(name: &str, repo_type: RepositoryType) -> Option<RepositoryConfig> {
        Self::get_repositories_by_type(repo_type)
            .into_iter()
            .find(|repo| repo.name.to_lowercase() == name.to_lowercase())
    }

    /// 获取镜像 URL
    pub fn get_mirror_url(original_url: &str, repo_type: RepositoryType) -> Option<String> {
        let repositories = Self::get_repositories_by_type(repo_type);

        // 简单的镜像逻辑：如果原 URL 是官方的，返回镜像 URL
        if repo_type == RepositoryType::Java {
            if original_url.contains("api.adoptium.net") {
                repositories
                    .iter()
                    .find(|repo| repo.name.contains("Mirror"))
                    .map(|repo| repo.url.clone())
            } else {
                None
            }
        } else {
            None
        }
    }
}