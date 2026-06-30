use crate::core::environment_manager::EnvironmentType;
use crate::infrastructure::shell::ShellType;
use clap::{Command, CommandFactory, Parser, Subcommand};

/// fnva CLI application
#[derive(Parser)]
#[command(name = "fnva")]
#[command(about = "Cross-platform environment switcher for Java / Maven / Claude Code", long_about = None)]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// 重写 command 方法，为 --version 添加 -v 别名
    pub fn command() -> Command {
        <Self as CommandFactory>::command()
            .version(env!("CARGO_PKG_VERSION"))
            .disable_version_flag(true)
            .arg(
                clap::Arg::new("version")
                    .long("version")
                    .short('V')
                    .visible_short_alias('v')
                    .action(clap::ArgAction::Version)
                    .help("Print version information"),
            )
    }
}

/// Top-level commands
#[derive(Subcommand)]
pub enum Commands {
    /// Manage Java environments
    Java {
        #[command(subcommand)]
        action: JavaCommands,
    },
    /// Manage Maven environments
    Maven {
        #[command(subcommand)]
        action: MavenCommands,
    },
    /// Manage CC (Claude Code) environments
    Cc {
        #[command(subcommand)]
        action: CcCommands,
    },
    /// Switch and manage environments
    Env {
        #[command(subcommand)]
        action: EnvCommands,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Environment history
    History {
        /// Environment type
        #[arg(short, long)]
        env_type: Option<String>,
        /// Result limit
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Java environment management commands
#[derive(Subcommand)]
pub enum JavaCommands {
    /// List Java environments
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Switch to a Java environment
    Use {
        /// Environment name
        name: String,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
        /// Output format
        #[arg(long)]
        json: bool,
    },
    /// Scan the system for Java installations
    Scan,
    /// Add a Java environment
    Add {
        /// Environment name
        #[arg(short, long)]
        name: String,
        /// JAVA_HOME path
        #[arg(long)]
        home: String,
        /// Description
        #[arg(short = 'd', long)]
        description: Option<String>,
    },
    /// Remove a Java environment
    Remove {
        /// Environment name
        name: String,
    },
    /// Query available remote versions
    LsRemote {
        /// Query type
        #[arg(default_value = "java")]
        query_type: String,
        /// Major Java version
        #[arg(long)]
        java_version: Option<u32>,
        /// Repository URL
        #[arg(long)]
        repository: Option<String>,
        /// Result limit
        #[arg(short = 'n', long, default_value = "20")]
        limit: u32,
    },
    /// Install a Java version
    Install {
        /// Java version
        version: String,
        /// Auto-switch after install
        #[arg(long)]
        auto_switch: bool,
    },
    /// Uninstall a Java version
    Uninstall {
        /// Java environment name
        name: String,
    },
    /// Set or show the default Java environment
    Default {
        /// Java environment name (shows current default when omitted)
        name: Option<String>,
        /// Clear the default
        #[arg(long)]
        unset: bool,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show the current Java environment
    Current {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Maven environment management commands
#[derive(Subcommand)]
pub enum MavenCommands {
    /// List Maven environments
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Switch to a Maven environment
    Use {
        /// Environment name
        name: String,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Install a Maven version
    Install {
        /// Version (e.g. 3.9.16 / latest / 3.9)
        version: String,
        /// Auto-switch to the environment after install
        #[arg(long)]
        auto_switch: bool,
    },
    /// Uninstall a Maven version
    Uninstall {
        /// Environment name
        name: String,
    },
    /// Refresh the remote version cache
    Refresh,
    /// List available remote versions
    LsRemote {
        /// Version prefix filter (e.g. 3.9)
        #[arg(long)]
        version: Option<String>,
    },
    /// Show the current Maven environment
    Current {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set or show the default Maven environment
    Default {
        /// Environment name (shows current default when omitted)
        name: Option<String>,
        /// Clear the default
        #[arg(long)]
        unset: bool,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// CC (Claude Code) environment management commands
#[derive(Subcommand)]
pub enum CcCommands {
    /// List CC environments
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Switch to a CC environment
    Use {
        /// Environment name
        name: String,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Add a CC environment
    Add {
        /// Environment name
        #[arg(short, long)]
        name: String,
        /// Provider
        #[arg(short, long)]
        provider: String,
        /// API Key
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        /// Base URL
        #[arg(short = 'u', long)]
        base_url: Option<String>,
        /// Model name
        #[arg(short, long)]
        model: Option<String>,
        /// Description
        #[arg(short = 'd', long)]
        description: Option<String>,
    },
    /// Remove a CC environment
    Remove {
        /// Environment name
        name: String,
    },
    /// Manage the default CC environment
    Default {
        /// CC environment name (shows current default when omitted)
        name: Option<String>,
        /// Clear the default
        #[arg(long)]
        unset: bool,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show the current CC environment
    Current {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

/// Environment management commands
#[derive(Subcommand)]
pub enum EnvCommands {
    /// Generate the environment switch script (like fnm env)
    #[command(name = "env")]
    GenerateEnv {
        /// Auto environment switch integration (like fnm env --use-on-cd)
        #[arg(long)]
        use_on_cd: bool,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
    },
    /// Switch environment
    Switch {
        /// Environment type
        #[arg(short = 't', long)]
        env_type: String,
        /// Environment name
        #[arg(short, long)]
        name: String,
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
        /// Switch reason
        #[arg(long)]
        reason: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List environments
    List {
        /// Environment type
        #[arg(short = 't', long)]
        env_type: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show the current environment
    Current {
        /// Environment type
        #[arg(short = 't', long)]
        env_type: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Generate the shell integration script
    ShellIntegration {
        /// Shell type
        #[arg(short, long)]
        shell: Option<String>,
    },
}

/// Configuration management commands
#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Complete and sync the configuration file
    Sync,
}

/// 解析环境类型字符串
pub fn parse_environment_type(env_type_str: &str) -> Result<EnvironmentType, String> {
    match env_type_str.to_lowercase().as_str() {
        "java" => Ok(EnvironmentType::Java),
        "cc" => Ok(EnvironmentType::Cc),
        "maven" => Ok(EnvironmentType::Maven),
        other => Err(format!(
            "Unsupported environment type: '{other}'. Supported: java, cc, maven"
        )),
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
