use clap::{Parser, Subcommand};
use crate::core::environment_manager::EnvironmentType;
use crate::infrastructure::shell::ShellType;

/// fnva CLI 应用程序
#[derive(Parser)]
#[command(name = "fnva")]
#[command(about = "跨平台环境切换工具，支持 Java 和 LLM 环境配置", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// 顶级命令
#[derive(Subcommand)]
pub enum Commands {
    /// Java 环境管理
    Java {
        #[command(subcommand)]
        action: JavaCommands,
    },
    /// LLM 环境管理
    Llm {
        #[command(subcommand)]
        action: LlmCommands,
    },
    /// 环境切换和管理
    Env {
        #[command(subcommand)]
        action: EnvCommands,
    },
    /// 网络连接诊断
    NetworkTest,
    /// 环境历史
    History {
        /// 环境类型
        #[arg(short, long)]
        env_type: Option<String>,
        /// 显示数量限制
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
}

/// Java 环境管理命令
#[derive(Subcommand)]
pub enum JavaCommands {
    /// 列出所有 Java 环境
    List {
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 切换到指定的 Java 环境
    Use {
        /// 环境名称
        name: String,
        /// Shell 类型
        #[arg(short, long)]
        shell: Option<String>,
        /// 输出格式
        #[arg(long)]
        json: bool,
    },
    /// 使用指定 Java 版本执行命令
    Run {
        /// 环境名称
        name: String,
        /// Java 命令参数
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// 扫描系统中的 Java 安装
    Scan,
    /// 添加 Java 环境
    Add {
        /// 环境名称
        #[arg(short, long)]
        name: String,
        /// JAVA_HOME 路径
        #[arg(long)]
        home: String,
        /// 描述
        #[arg(short = 'd', long)]
        description: Option<String>,
    },
    /// 删除 Java 环境
    Remove {
        /// 环境名称
        name: String,
    },
    /// 远程查询可用版本
    LsRemote {
        /// 查询类型
        #[arg(default_value = "java")]
        query_type: String,
        /// Java 主要版本
        #[arg(long)]
        java_version: Option<u32>,
        /// Maven Group ID
        #[arg(long)]
        maven_artifact: Option<String>,
        /// 搜索关键词
        #[arg(long)]
        search: Option<String>,
        /// 仓库 URL
        #[arg(long)]
        repository: Option<String>,
        /// 结果数量限制
        #[arg(short = 'n', long, default_value = "20")]
        limit: u32,
    },
    /// 安装 Java 版本
    Install {
        /// Java 版本
        version: String,
        /// 安装后自动切换
        #[arg(long)]
        auto_switch: bool,
    },
    /// 卸载 Java 版本
    Uninstall {
        /// Java 环境名称
        name: String,
    },
    /// 显示当前激活的 Java 环境
    Current {
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 安装 Shell 集成
    ShellInstall,
    /// 安装 Shell Hook
    InstallHook,
    /// 卸载 Shell Hook
    UninstallHook,
    /// 列出可安装的 Java 版本
    ListInstallable,
}

/// LLM 环境管理命令
#[derive(Subcommand)]
pub enum LlmCommands {
    /// 列出所有 LLM 环境
    List {
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 切换到指定的 LLM 环境
    Use {
        /// 环境名称
        name: String,
        /// Shell 类型
        #[arg(short, long)]
        shell: Option<String>,
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 添加 LLM 环境
    Add {
        /// 环境名称
        #[arg(short, long)]
        name: String,
        /// 提供商
        #[arg(short, long)]
        provider: String,
        /// API Key
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        /// Base URL
        #[arg(short = 'u', long)]
        base_url: Option<String>,
        /// 模型名称
        #[arg(short, long)]
        model: Option<String>,
        /// Temperature
        #[arg(short = 't', long)]
        temperature: Option<f64>,
        /// Max tokens
        #[arg(short = 'm', long)]
        max_tokens: Option<u32>,
        /// 描述
        #[arg(short = 'd', long)]
        description: Option<String>,
    },
    /// 删除 LLM 环境
    Remove {
        /// 环境名称
        name: String,
    },
    /// 列出支持的提供商
    Providers,
    /// 显示当前激活的 LLM 环境
    Current {
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
}

/// 环境管理命令
#[derive(Subcommand)]
pub enum EnvCommands {
    /// 自动环境切换集成
    UseOnCd {
        /// Shell 类型
        #[arg(short, long)]
        shell: Option<String>,
    },
    /// 切换环境
    Switch {
        /// 环境类型
        #[arg(short = 't', long)]
        env_type: String,
        /// 环境名称
        #[arg(short, long)]
        name: String,
        /// Shell 类型
        #[arg(short, long)]
        shell: Option<String>,
        /// 切换原因
        #[arg(long)]
        reason: Option<String>,
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 列出环境
    List {
        /// 环境类型
        #[arg(short = 't', long)]
        env_type: Option<String>,
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 添加环境
    Add {
        /// 环境类型
        #[arg(short = 't', long)]
        env_type: String,
        /// 环境名称
        #[arg(short, long)]
        name: String,
        /// 配置文件路径
        #[arg(long)]
        config: Option<String>,
        /// 交互式配置
        #[arg(long)]
        interactive: bool,
    },
    /// 删除环境
    Remove {
        /// 环境类型
        #[arg(short = 't', long)]
        env_type: String,
        /// 环境名称
        #[arg(short, long)]
        name: String,
    },
    /// 获取当前环境
    Current {
        /// 环境类型
        #[arg(short = 't', long)]
        env_type: Option<String>,
        /// JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 扫描环境
    Scan {
        /// 环境类型
        #[arg(short = 't', long)]
        env_type: String,
    },
    /// 生成 shell 集成脚本
    ShellIntegration {
        /// Shell 类型
        #[arg(short, long)]
        shell: Option<String>,
    },
}

/// 解析环境类型字符串
pub fn parse_environment_type(env_type_str: &str) -> Result<EnvironmentType, String> {
    match env_type_str.to_lowercase().as_str() {
        "java" => Ok(EnvironmentType::Java),
        "llm" => Ok(EnvironmentType::Llm),
        "maven" => Ok(EnvironmentType::Maven),
        "gradle" => Ok(EnvironmentType::Gradle),
        "python" => Ok(EnvironmentType::Python),
        "node" | "nodejs" => Ok(EnvironmentType::Node),
        _ => Err(format!("Unsupported environment type: {}", env_type_str)),
    }
}

/// 解析 Shell 类型字符串
pub fn parse_shell_type(shell_str: &str) -> Result<ShellType, String> {
    match shell_str.to_lowercase().as_str() {
        "bash" => Ok(ShellType::Bash),
        "zsh" => Ok(ShellType::Zsh),
        "fish" => Ok(ShellType::Fish),
        "powershell" | "ps1" => Ok(ShellType::PowerShell),
        "cmd" => Ok(ShellType::Cmd),
        _ => Ok(ShellType::Unknown),
    }
}