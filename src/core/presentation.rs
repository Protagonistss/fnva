//! core 层的展示数据模型:输出格式 + 列表/历史数据项。
//!
//! 这些是纯数据(无 ANSI 颜色/渲染),由 core 层(switcher)产生、cli 层
//! (`print::format_envs`/`format_history`)消费渲染。下沉到 core 是为了消除
//! core → cli 的反向依赖。

use serde::Serialize;

/// 输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

/// 环境列表的一个条目(供 cli 层 `format_envs` 渲染,或 Json 序列化)。
#[derive(Serialize, Clone, Debug)]
pub struct EnvItem {
    pub name: String,
    pub description: String,
    pub extra: Option<String>,
    pub is_current: bool,
    pub is_default: bool,
    /// 缺少必要凭据(CC 没配 api_key)→ 渲染成 `⚠ no key` 标签,提醒该环境导出后无法鉴权。
    pub missing_key: bool,
}

/// 切换历史的一个条目(供 cli 层 `format_history` 渲染)。
#[derive(Serialize, Clone, Debug)]
pub struct HistoryItem {
    pub timestamp: String,
    pub env_type: String,
    pub from: Option<String>,
    pub to: String,
}

/// 扫描结果项(供 switcher 打印;各 scanner 自带 import 命令)。
///
/// - `location`:安装路径(Java/Maven)或 base_url(CC)
/// - `detail`:展示用的版本 / model 等附加信息
/// - `import_cmd`:该环境对应的 `fnva <type> add ...` 命令(由 scanner 生成)
pub struct ScanHit {
    pub name: String,
    pub location: String,
    pub detail: String,
    pub import_cmd: String,
}
