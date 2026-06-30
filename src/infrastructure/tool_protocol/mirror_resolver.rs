use crate::infrastructure::config::MirrorConfig;
use crate::infrastructure::remote::mirror_utils::is_url_available_with_timeout;
use reqwest::Client;
use std::fmt;
use std::time::Duration;

use super::template_vars::TemplateVars;

/// 镜像解析失败
#[derive(Debug)]
pub enum ResolveError {
    /// 所有启用的镜像均不可用(HEAD 探测全部失败)
    AllUnavailable,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResolveError::AllUnavailable => write!(f, "All mirrors unavailable"),
        }
    }
}

impl std::error::Error for ResolveError {}

/// 工具无关的「模板渲染 + HEAD 探测 + 按优先级回退」解析器。
///
/// 取代 `template_downloader.rs` 中 Java 专属的 `get_download_url` 镜像遍历逻辑。
/// 构造时按 `priority` 升序排序镜像;`resolve` 依次渲染模板并做 5s HEAD 探测,
/// 返回首个可用的下载 URL。
pub struct MirrorResolver {
    client: Client,
    mirrors: Vec<MirrorConfig>,
}

impl MirrorResolver {
    pub fn new(mut mirrors: Vec<MirrorConfig>) -> Self {
        mirrors.sort_by_key(|m| m.priority);
        Self {
            client: Client::new(),
            mirrors,
        }
    }

    /// 底层 HTTP 客户端(供下载器复用同一连接池)
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// 优先级最高的镜像名(用于缓存文件命名等)
    pub fn first_mirror_name(&self) -> &str {
        self.mirrors
            .first()
            .map(|m| m.name.as_str())
            .unwrap_or("unknown")
    }

    /// 按 priority 遍历启用的镜像 → 渲染 → HEAD 探测(5s)→ 返回首个可用 URL。
    pub async fn resolve(&self, vars: &TemplateVars) -> Result<String, ResolveError> {
        for mirror in &self.mirrors {
            if !mirror.enabled {
                continue;
            }
            let url = TemplateVars::render(&mirror.url_template, &mirror.base_url, vars);
            if is_url_available_with_timeout(&self.client, &url, Duration::from_secs(5)).await {
                return Ok(url);
            }
        }
        Err(ResolveError::AllUnavailable)
    }
}
