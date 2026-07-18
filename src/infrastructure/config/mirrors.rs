use serde::{Deserialize, Serialize};

/// 镜像配置（模板化 URL）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub name: String,
    #[serde(default = "default_mirror_priority")]
    pub priority: u32,
    pub base_url: String,
    /// URL 模板变量: {base_url}, {major}, {tag}, {filename}, {os}, {arch}
    pub url_template: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_mirror_priority() -> u32 {
    10
}

fn default_true() -> bool {
    true
}

/// 所有工具的镜像配置集合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorsConfig {
    #[serde(default = "default_java_mirrors")]
    pub java: Vec<MirrorConfig>,
    #[serde(default = "default_maven_mirrors")]
    pub maven: Vec<MirrorConfig>,
}

impl Default for MirrorsConfig {
    fn default() -> Self {
        Self {
            java: default_java_mirrors(),
            maven: default_maven_mirrors(),
        }
    }
}

impl MirrorsConfig {
    /// 通用访问器:按工具 id 取镜像列表(未知工具返回空切片)。
    pub fn get(&self, tool: &str) -> &[MirrorConfig] {
        match tool {
            "java" => &self.java,
            "maven" => &self.maven,
            _ => &[],
        }
    }
}

fn default_maven_mirrors() -> Vec<MirrorConfig> {
    vec![
        MirrorConfig {
            name: "tsinghua".to_string(),
            priority: 1,
            base_url: "https://mirrors.tuna.tsinghua.edu.cn/apache/maven/maven-3".to_string(),
            url_template: "{base_url}/{version}/binaries/apache-maven-{version}-bin.tar.gz"
                .to_string(),
            enabled: true,
        },
        MirrorConfig {
            name: "apache-archive".to_string(),
            priority: 2,
            base_url: "https://archive.apache.org/dist/maven/maven-3".to_string(),
            url_template: "{base_url}/{version}/binaries/apache-maven-{version}-bin.tar.gz"
                .to_string(),
            enabled: true,
        },
    ]
}

fn default_java_mirrors() -> Vec<MirrorConfig> {
    vec![
        MirrorConfig {
            name: "tsinghua".to_string(),
            priority: 1,
            base_url: "https://mirrors.tuna.tsinghua.edu.cn/Adoptium".to_string(),
            url_template: "{base_url}/{major}/jdk/{arch}/{os}/{filename}".to_string(),
            enabled: true,
        },
        MirrorConfig {
            name: "github".to_string(),
            priority: 2,
            base_url: String::new(),
            url_template: "https://github.com/adoptium/temurin{major}-binaries/releases/download/{tag}/{filename}".to_string(),
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_maven_mirrors() {
        let m = default_maven_mirrors();
        assert!(m.iter().any(|x| x.name == "tsinghua"));
        assert!(m.iter().any(|x| x.name == "apache-archive"));
        // 清华优先级最高
        assert_eq!(
            m.iter().min_by_key(|x| x.priority).unwrap().name,
            "tsinghua"
        );
    }
}
