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
}

/// 切换历史的一个条目(供 cli 层 `format_history` 渲染)。
#[derive(Serialize, Clone, Debug)]
pub struct HistoryItem {
    pub timestamp: String,
    pub env_type: String,
    pub from: Option<String>,
    pub to: String,
}
