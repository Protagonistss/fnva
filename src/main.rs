use clap::{Parser, Subcommand};
use fnva::config::Config;
use fnva::java::JavaManager;
use fnva::llm::LlmManager;
use fnva::platform::ShellType;
use std::process;

#[derive(Parser)]
#[command(name = "fnva")]
#[command(about = "跨平台环境切换工具，支持 Java 和 LLM 环境配置", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
}

#[derive(Subcommand)]
enum JavaCommands {
    /// 列出所有 Java 环境
    List,
    /// 切换到指定的 Java 环境
    Use {
        /// 环境名称
        name: String,
        /// Shell 类型 (bash, zsh, fish, powershell, cmd)
        #[arg(short, long)]
        shell: Option<String>,
    },
    /// 扫描系统中的 Java 安装
    Scan,
    /// 添加 Java 环境
    Add {
        /// 环境名称
        #[arg(short, long)]
        name: String,
        /// JAVA_HOME 路径
        #[arg(short, long)]
        home: String,
        /// 描述
        #[arg(short, long)]
        description: Option<String>,
    },
    /// 删除 Java 环境
    Remove {
        /// 环境名称
        name: String,
    },
}

#[derive(Subcommand)]
enum LlmCommands {
    /// 列出所有 LLM 环境
    List,
    /// 切换到指定的 LLM 环境
    Use {
        /// 环境名称
        name: String,
        /// Shell 类型 (bash, zsh, fish, powershell, cmd)
        #[arg(short, long)]
        shell: Option<String>,
    },
    /// 添加 LLM 环境
    Add {
        /// 环境名称
        #[arg(short, long)]
        name: String,
        /// 提供商 (openai, anthropic, azure-openai, google-gemini, cohere, mistral, ollama)
        #[arg(short, long)]
        provider: String,
        /// API Key（支持 ${VAR_NAME} 格式）
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        /// Base URL
        #[arg(short = 'u', long)]
        base_url: Option<String>,
        /// 模型名称
        #[arg(short, long)]
        model: Option<String>,
        /// Temperature (0.0-2.0)
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
}

fn parse_shell(shell_str: Option<String>) -> Option<ShellType> {
    shell_str.map(|s| match s.to_lowercase().as_str() {
        "bash" => ShellType::Bash,
        "zsh" => ShellType::Zsh,
        "fish" => ShellType::Fish,
        "powershell" | "ps1" => ShellType::PowerShell,
        "cmd" => ShellType::Cmd,
        _ => ShellType::Unknown,
    })
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Java { action } => handle_java_command(action),
        Commands::Llm { action } => handle_llm_command(action),
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn handle_java_command(action: JavaCommands) -> Result<(), String> {
    match action {
        JavaCommands::List => {
            let config = Config::load()?;
            let envs = JavaManager::list(&config);
            
            if envs.is_empty() {
                println!("没有配置的 Java 环境");
                println!("\n使用 'fnva java scan' 扫描系统中的 Java 安装");
                println!("或使用 'fnva java add' 手动添加");
            } else {
                println!("已配置的 Java 环境:");
                for env in envs {
                    println!("  {}: {}", env.name, env.java_home);
                    if !env.description.is_empty() {
                        println!("    描述: {}", env.description);
                    }
                }
            }
            Ok(())
        }
        JavaCommands::Use { name, shell } => {
            let config = Config::load()?;
            let shell_type = parse_shell(shell);
            let command = JavaManager::generate_switch_command(&config, &name, shell_type)?;
            println!("{}", command);
            Ok(())
        }
        JavaCommands::Scan => {
            println!("正在扫描系统中的 Java 安装...");
            let installations = JavaManager::scan_system();
            
            if installations.is_empty() {
                println!("未找到 Java 安装");
            } else {
                println!("\n找到 {} 个 Java 安装:", installations.len());
                for (i, install) in installations.iter().enumerate() {
                    println!("  {}. {}", i + 1, install.description);
                    if let Some(version) = &install.version {
                        println!("     版本: {}", version);
                    }
                    println!("     JAVA_HOME: {}", install.java_home);
                }
                println!("\n使用以下命令添加环境:");
                println!("  fnva java add --name <名称> --home <JAVA_HOME路径>");
            }
            Ok(())
        }
        JavaCommands::Add {
            name,
            home,
            description,
        } => {
            let mut config = Config::load()?;
            JavaManager::add(&mut config, name.clone(), home.clone(), description)?;
            println!("已添加 Java 环境: {}", name);
            println!("  JAVA_HOME: {}", home);
            Ok(())
        }
        JavaCommands::Remove { name } => {
            let mut config = Config::load()?;
            JavaManager::remove(&mut config, &name)?;
            println!("已删除 Java 环境: {}", name);
            Ok(())
        }
    }
}

fn handle_llm_command(action: LlmCommands) -> Result<(), String> {
    match action {
        LlmCommands::List => {
            let config = Config::load()?;
            let envs = LlmManager::list(&config);
            
            if envs.is_empty() {
                println!("没有配置的 LLM 环境");
                println!("\n使用 'fnva llm add' 添加 LLM 环境");
            } else {
                println!("已配置的 LLM 环境:");
                for env in envs {
                    println!("  {} ({})", env.name, env.provider);
                    if !env.description.is_empty() {
                        println!("    描述: {}", env.description);
                    }
                    if !env.model.is_empty() {
                        println!("    模型: {}", env.model);
                    }
                    if !env.base_url.is_empty() {
                        println!("    Base URL: {}", env.base_url);
                    }
                }
            }
            Ok(())
        }
        LlmCommands::Use { name, shell } => {
            let config = Config::load()?;
            let shell_type = parse_shell(shell);
            let command = LlmManager::generate_switch_command(&config, &name, shell_type)?;
            println!("{}", command);
            Ok(())
        }
        LlmCommands::Add {
            name,
            provider,
            api_key,
            base_url,
            model,
            temperature,
            max_tokens,
            description,
        } => {
            // 验证提供商
            let providers = LlmManager::get_providers();
            if !providers.contains(&provider.as_str()) {
                return Err(format!(
                    "不支持的提供商: {}. 支持的提供商: {}",
                    provider,
                    providers.join(", ")
                ));
            }

            let mut config = Config::load()?;
            LlmManager::add(
                &mut config,
                name.clone(),
                provider.clone(),
                api_key,
                base_url,
                model,
                temperature,
                max_tokens,
                description,
            )?;
            println!("已添加 LLM 环境: {} ({})", name, provider);
            Ok(())
        }
        LlmCommands::Remove { name } => {
            let mut config = Config::load()?;
            LlmManager::remove(&mut config, &name)?;
            println!("已删除 LLM 环境: {}", name);
            Ok(())
        }
        LlmCommands::Providers => {
            let providers = LlmManager::get_providers();
            println!("支持的 LLM 提供商:");
            for provider in providers {
                println!("  - {}", provider);
            }
            Ok(())
        }
    }
}
