use clap::{Parser, Subcommand};
use fnva::config::Config;
use fnva::installer::JavaInstaller;
use fnva::java::JavaManager;
use fnva::llm::LlmManager;
use fnva::network_test::NetworkTester;
use fnva::package_manager::JavaPackageManager;
use fnva::platform::{detect_shell, ShellType};
use fnva::remote::RemoteManager;
use fnva::shell_hook::ShellHook;
use fnva::shell_integration::ShellIntegration;
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
    /// 网络连接诊断
    NetworkTest,
    /// Shell �������ű�
    Env {
        /// ��ÿ�θı�Ŀ¼ʱ�Զ���ȡ��ǰ����
        #[arg(long = "use-on-cd")]
        use_on_cd: bool,
        /// ָ�� shell ���� (bash, zsh, fish, powershell, cmd)
        #[arg(short, long)]
        shell: Option<String>,
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
        /// 查询类型 (java, maven)
        #[arg(default_value = "java")]
        query_type: String,
        /// Java 主要版本 (仅用于 java 查询)
        #[arg(long)]
        java_version: Option<u32>,
        /// Maven Group ID (格式: groupId:artifactId)
        #[arg(long)]
        maven_artifact: Option<String>,
        /// 搜索关键词 (用于搜索 Maven 工件)
        #[arg(long)]
        search: Option<String>,
        /// 仓库 URL (可选，使用配置中的默认仓库)
        #[arg(long)]
        repository: Option<String>,
        /// 结果数量限制
        #[arg(short = 'n', long, default_value = "20")]
        limit: u32,
    },
    /// 安装 Java 版本（下载资源包）
    Install {
        /// Java 版本 (支持格式: v21, 21, java21, jdk21)
        version: String,
        /// 安装后自动切换到该版本
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
        /// 以 JSON 格式输出
        #[arg(long)]
        json: bool,
    },
    /// 安装 Shell 集成
    ShellInstall,
    /// 安装 Shell Hook（实现当前 shell 立即生效）
    InstallHook,
    /// 卸载 Shell Hook
    UninstallHook,
    /// 列出可安装的 Java 版本
    ListInstallable,
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
    // 自动激活当前环境
    if let Err(e) = auto_activate_current_environment() {
        // 静默失败，不影响主要功能
        eprintln!("警告: 自动激活环境失败: {}", e);
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Java { action } => handle_java_command(action),
        Commands::Llm { action } => handle_llm_command(action),
        Commands::NetworkTest => handle_network_test(),
        Commands::Env { use_on_cd, shell } => handle_env_command(use_on_cd, shell),
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn handle_env_command(use_on_cd: bool, shell: Option<String>) -> Result<(), String> {
    if !use_on_cd {
        return Err("Only --use-on-cd is supported at the moment".to_string());
    }

    let shell_type = parse_shell(shell).unwrap_or_else(detect_shell);
    let script = ShellHook::generate_use_on_cd_script(shell_type)?;
    println!("{}", script);
    Ok(())
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
            let mut config = Config::load()?;
            let shell_type = parse_shell(shell);
            let switch_commands =
                JavaManager::generate_switch_command(&config, &name, shell_type)?;

            config.set_current_java_env(name.clone())?;
            config.save()?;
            ShellHook::set_current_environment(&name)?;

            println!("{}", switch_commands);
            Ok(())
        }
        JavaCommands::Run { name, args } => {
            let config = Config::load()?;
            JavaManager::execute_with_java(&config, &name, args)?;
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
        JavaCommands::LsRemote {
            query_type,
            java_version,
            maven_artifact,
            search,
            repository,
            limit,
        } => {
            let config = Config::load()?;
            use tokio::runtime::Runtime;

            let rt = Runtime::new().map_err(|e| format!("创建异步运行时失败: {}", e))?;

            rt.block_on(async {
                match query_type.as_str() {
                    "java" => {
                        let repo_url = repository
                            .or_else(|| config.repositories.java.first().cloned())
                            .unwrap_or_else(|| "https://api.adoptium.net/v3".to_string());

                        match RemoteManager::list_java_versions(
                            &repo_url,
                            java_version,
                            None,
                            None,
                        )
                        .await
                        {
                            Ok(versions) => {
                                if versions.is_empty() {
                                    println!("未找到可用的 Java 版本");
                                } else {
                                    println!(
                                        "可用的 Java 版本 (显示前 {} 个):",
                                        std::cmp::min(limit, versions.len() as u32)
                                    );
                                    for (i, version) in
                                        versions.iter().take(limit as usize).enumerate()
                                    {
                                        println!(
                                            "  {}. Java {} ({})",
                                            i + 1,
                                            version.version,
                                            version.release_name
                                        );
                                        if let Some(download_url) = &version.download_url {
                                            println!("     下载: {}", download_url);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                return Err(format!("查询 Java 版本失败: {}", e));
                            }
                        }
                    }
                    "maven" => {
                        let repo_url = repository
                            .or_else(|| config.repositories.maven.first().cloned())
                            .unwrap_or_else(|| {
                                "https://search.maven.org/solrsearch/select".to_string()
                            });

                        if let Some(search_query) = search {
                            match RemoteManager::search_maven_artifacts(
                                &repo_url,
                                &search_query,
                                Some(limit),
                            )
                            .await
                            {
                                Ok(artifacts) => {
                                    if artifacts.is_empty() {
                                        println!("未找到匹配的 Maven 工件");
                                    } else {
                                        println!("搜索结果 (共 {} 条):", artifacts.len());
                                        for (i, artifact) in artifacts.iter().enumerate() {
                                            println!(
                                                "  {}. {}:{}",
                                                i + 1,
                                                artifact.group_id,
                                                artifact.artifact_id
                                            );
                                            println!("     最新版本: {}", artifact.latest_version);
                                            println!("     打包类型: {}", artifact.packaging);
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(format!("搜索 Maven 工件失败: {}", e));
                                }
                            }
                        } else if let Some(artifact) = maven_artifact {
                            let parts: Vec<&str> = artifact.split(':').collect();
                            if parts.len() != 2 {
                                return Err("Maven 工件格式应为 'groupId:artifactId'".to_string());
                            }

                            let group_id = parts[0];
                            let artifact_id = parts[1];

                            match RemoteManager::list_maven_versions(
                                &repo_url,
                                group_id,
                                artifact_id,
                            )
                            .await
                            {
                                Ok(versions) => {
                                    if versions.is_empty() {
                                        println!("未找到该依赖的可用版本");
                                    } else {
                                        println!(
                                            "{}:{} 的可用版本 (显示前 {} 个):",
                                            group_id,
                                            artifact_id,
                                            std::cmp::min(limit, versions.len() as u32)
                                        );
                                        for (i, version) in
                                            versions.iter().take(limit as usize).enumerate()
                                        {
                                            println!(
                                                "  {}. {} ({})",
                                                i + 1,
                                                version.version,
                                                version.packaging
                                            );
                                        }
                                    }
                                }
                                Err(e) => {
                                    return Err(format!("查询 Maven 版本失败: {}", e));
                                }
                            }
                        } else {
                            return Err(
                                "查询 Maven 版本需要指定 --maven-artifact 或 --search 参数".to_string()
                            );
                        }
                    }
                    _ => {
                        return Err("不支持的查询类型。支持的类型: java, maven".to_string());
                    }
                }

                Ok(())
            })
        }
        JavaCommands::Install { version, auto_switch } => {
            use tokio::runtime::Runtime;

            let mut config = Config::load()?;
            let rt = Runtime::new().map_err(|e| format!("创建异步运行时失败: {}", e))?;

            rt.block_on(async {
                match JavaPackageManager::install_java_package(&version, &mut config, auto_switch)
                    .await
                {
                    Ok(java_home) => {
                        println!("🎉 Java {} 资源包安装完成", version);
                        println!("📍 JAVA_HOME: {}", java_home);
                        println!("💡 使用 'fnva java use {}' 来切换到此版本", version);
                        println!("🌟 使用阿里云镜像源，下载更快更稳定！");
                    }
                    Err(e) => {
                        return Err(format!("安装 Java {} 资源包失败: {}", version, e));
                    }
                }

                Ok(())
            })
        }
        JavaCommands::Uninstall { name } => {
            let mut config = Config::load()?;

            if name.starts_with("jdk-pkg-") {
                JavaPackageManager::uninstall_java_package(&name, &mut config)?;
            } else {
                JavaInstaller::uninstall_java(&name, &mut config)?;
            }
            Ok(())
        }
        JavaCommands::Current { json } => {
            let config = Config::load()?;
            if let Some(current) = &config.current_java_env {
                if json {
                    if let Some(env) = config.get_java_env(current) {
                        let output = serde_json::json!({
                            "name": current,
                            "java_home": env.java_home,
                            "description": env.description
                        });
                        println!("{}", serde_json::to_string_pretty(&output).unwrap());
                    } else {
                        let output = serde_json::json!({
                            "name": current,
                            "java_home": null,
                            "description": ""
                        });
                        println!("{}", serde_json::to_string_pretty(&output).unwrap());
                    }
                } else {
                    println!("当前 Java 环境: {}", current);
                    if let Some(env) = config.get_java_env(current) {
                        println!("  JAVA_HOME: {}", env.java_home);
                        if !env.description.is_empty() {
                            println!("  描述: {}", env.description);
                        }
                    }
                }
            } else {
                if json {
                    let output = serde_json::json!({
                        "name": null,
                        "java_home": null,
                        "description": null
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                } else {
                    println!("当前没有已激活的 Java 环境");
                }
            }
            Ok(())
        }
        JavaCommands::ShellInstall => {
            let integration_info = ShellIntegration::generate_shell_integration()?;
            println!("{}", integration_info);
            Ok(())
        }
        JavaCommands::InstallHook => {
            let hook_info = ShellHook::generate_hook_installation()?;
            println!("{}", hook_info);
            Ok(())
        }
        JavaCommands::UninstallHook => {
            let uninstall_info = ShellHook::generate_hook_uninstallation()?;
            println!("{}", uninstall_info);
            Ok(())
        }
        JavaCommands::ListInstallable => {
            use tokio::runtime::Runtime;

            let rt = Runtime::new().map_err(|e| format!("创建异步运行时失败: {}", e))?;

            rt.block_on(async {
                match JavaPackageManager::list_installable_packages().await {
                    Ok(packages) => {
                        if packages.is_empty() {
                            println!("没有可安装的 Java 版本");
                        } else {
                            println!("可安装的 Java 版本（资源包模式）:");
                            for package in packages {
                                println!("  {}", package);
                            }
                            println!("\n💡 使用 'fnva java install v21' 来安装资源包版本");
                            println!("🌟 资源包模式特色:");
                            println!("   ✅ 使用阿里云镜像源，下载更快");
                            println!("   ✅ 无需管理员权限");
                            println!("   ✅ 下载便携式版本");
                            println!("   ✅ 解压即用");
                            println!("   ✅ 轻松卸载");
                            println!("   ✅ 完全隔离，不影响系统");
                        }
                    }
                    Err(e) => {
                        return Err(format!("获取可安装版本失败: {}", e));
                    }
                }

                Ok(())
            })
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

fn handle_network_test() -> Result<(), String> {
    use tokio::runtime::Runtime;

    let rt = Runtime::new().map_err(|e| format!("创建异步运行时失败: {}", e))?;

    rt.block_on(async {
        match NetworkTester::run_full_diagnosis().await {
            Ok(()) => {
                println!("\n💡 如果诊断发现问题，请查看 NETWORK_TROUBLESHOOTING.md 获取解决方案");
                Ok(())
            }
            Err(e) => {
                println!("\n❌ 网络诊断失败: {}", e);

                // 提供解决建议
                let suggestions = NetworkTester::provide_suggestions(&e);
                if !suggestions.is_empty() {
                    println!("\n💡 建议的解决方案:");
                    for (i, suggestion) in suggestions.iter().enumerate() {
                        println!("  {}. {}", i + 1, suggestion);
                    }
                }

                Err(format!("网络诊断失败: {}", e))
            }
        }
    })
}

/// 自动激活当前配置的 Java 环境（使用 Hook 机制）
fn auto_activate_current_environment() -> Result<(), String> {
    // 使用 Shell Hook 机制检查并应用当前环境
    ShellHook::check_and_apply_current()
}
