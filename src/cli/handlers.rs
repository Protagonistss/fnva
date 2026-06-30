use crate::cli::commands::*;
use crate::cli::output::{OutputFormat, FORMATTER};
use crate::core::environment_manager::EnvironmentType;
use crate::core::switcher::EnvironmentSwitcher;
use crate::infrastructure::shell::platform::detect_shell;
use std::sync::{Arc, Mutex};

/// 命令处理器
pub struct CommandHandler {
    switcher: EnvironmentSwitcher,
}

impl CommandHandler {
    /// 创建新的命令处理器
    pub fn new() -> Result<Self, String> {
        // Migrate legacy flat layout to the grouped layout before anything else
        // (SessionManager and managers read from the new paths).
        crate::infrastructure::paths::migrate_layout();
        let mut switcher = EnvironmentSwitcher::new().map_err(|e| e.to_string())?;

        // 注册 Java 环境管理器
        let java_manager = crate::environments::java::JavaEnvironmentManager::new();
        switcher
            .register_manager(Arc::new(Mutex::new(java_manager)))
            .map_err(|e| e.to_string())?;

        // 注册 CC 环境管理器
        let cc_manager = crate::environments::cc::CcEnvironmentManager::new();
        switcher
            .register_manager(Arc::new(Mutex::new(cc_manager)))
            .map_err(|e| e.to_string())?;

        // 注册 Maven 环境管理器
        let maven_manager = crate::environments::maven::MavenEnvironmentManager::new();
        switcher
            .register_manager(Arc::new(Mutex::new(maven_manager)))
            .map_err(|e| e.to_string())?;

        Ok(Self { switcher })
    }

    /// 处理命令
    pub async fn handle_command(&mut self, command: Commands) -> Result<(), String> {
        match command {
            Commands::Java { action } => self.handle_java_command(action).await,
            Commands::Cc { action } => self.handle_cc_command(action).await,
            Commands::Maven { action } => self.handle_maven_command(action).await,
            Commands::Env { shell } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(detect_shell()),
                };
                let script = self
                    .switcher
                    .generate_shell_integration(shell_type.unwrap())
                    .await?;
                print!("{script}");
                Ok(())
            }
            Commands::Config { action } => self.handle_config_command(action).await,
            Commands::History {
                env_type,
                limit,
                json,
            } => self.handle_history_command(env_type, limit, json).await,
        }
    }

    /// 处理 Java 命令
    async fn handle_java_command(&mut self, action: JavaCommands) -> Result<(), String> {
        match action {
            JavaCommands::List { json } => {
                let output = self
                    .switcher
                    .list_environments_with_default(
                        EnvironmentType::Java,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            JavaCommands::Use { name, shell, json } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(crate::infrastructure::shell::platform::detect_shell()),
                };

                let result = self
                    .switcher
                    .switch_environment(
                        EnvironmentType::Java,
                        &name,
                        shell_type,
                        Some("Manual switch via command".to_string()),
                    )
                    .await?;

                // 对于 JSON 输出，格式化显示结果
                if json {
                    let output = FORMATTER.format_switch_result(&result, OutputFormat::Json)?;
                    print!("{output}");
                } else if result.success {
                    // 对于非 JSON 输出，直接输出切换脚本（类似 fnm 的行为）
                    if !result.script.is_empty() {
                        print!("{}", result.script);
                    } else {
                        // 如果没有脚本，显示成功消息
                        println!("Switched to Java environment: {name}");
                    }
                } else {
                    // 如果切换失败，显示错误信息
                    eprintln!(
                        "Failed to switch Java environment: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                    return Err("Environment switch failed".to_string());
                }
            }
            JavaCommands::Current { json } => {
                let output = self
                    .switcher
                    .get_current_environment(
                        EnvironmentType::Java,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            JavaCommands::Scan => {
                let output = self
                    .switcher
                    .scan_environments(EnvironmentType::Java)
                    .await?;
                print!("{output}");
            }
            JavaCommands::LsRemote {
                query_type,
                version,
                repository,
                limit: _,
            } => {
                if query_type == "java" {
                    // 使用新的版本管理器查询 Java 版本
                    let output = self.handle_java_ls_remote(version, repository).await?;
                    print!("{output}");
                } else {
                    return Err(format!("Query type '{query_type}' not supported"));
                }
            }
            JavaCommands::Install {
                version,
                auto_switch,
            } => {
                use crate::environments::java::installer::JavaInstaller;
                use crate::infrastructure::config::Config;

                let mut config = Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
                match JavaInstaller::install_java(&version, &mut config, auto_switch).await {
                    Ok(java_home) => {
                        println!("Java {version} installed");
                        println!("Path: {java_home}");
                    }
                    Err(e) => {
                        return Err(format!("Install failed: {e}"));
                    }
                }
            }
            JavaCommands::Add {
                name,
                home,
                description: _,
            } => {
                let config_value = serde_json::json!({
                    "java_home": home
                });
                let output = self
                    .switcher
                    .add_environment(EnvironmentType::Java, &name, config_value)
                    .await?;
                print!("{output}");
            }
            JavaCommands::Remove { name } => {
                let output = self
                    .switcher
                    .remove_environment(EnvironmentType::Java, &name)
                    .await?;
                print!("{output}");
            }
            JavaCommands::Uninstall { name } => {
                use crate::environments::java::installer::JavaInstaller;
                use crate::infrastructure::config::Config;

                let mut config = Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
                JavaInstaller::uninstall_java(&name, &mut config)?;
            }
            JavaCommands::Default {
                name,
                unset,
                shell,
                json,
            } => {
                if unset {
                    // 清除默认环境
                    let output = self
                        .switcher
                        .clear_default_environment(EnvironmentType::Java)
                        .await?;
                    print!("{output}");
                } else if let Some(env_name) = name {
                    // 设置默认环境
                    let output = self
                        .switcher
                        .set_default_environment(EnvironmentType::Java, &env_name)
                        .await?;
                    print!("{output}");
                } else {
                    // 显示当前默认环境
                    match self
                        .switcher
                        .get_default_environment(EnvironmentType::Java)
                        .await?
                    {
                        Some(env_name) => {
                            if let Some(shell) = shell {
                                match parse_shell_type(&shell) {
                                    Ok(shell_type) => {
                                        let result = self
                                            .switcher
                                            .switch_environment(
                                                EnvironmentType::Java,
                                                &env_name,
                                                Some(shell_type),
                                                Some("Switch to default environment".to_string()),
                                            )
                                            .await?;
                                        let output = FORMATTER.format_switch_result(
                                            &result,
                                            if json {
                                                OutputFormat::Json
                                            } else {
                                                OutputFormat::Text
                                            },
                                        )?;
                                        print!("{output}");
                                    }
                                    Err(e) => return Err(e),
                                }
                            } else {
                                println!("Default Java environment: {env_name}");
                            }
                        }
                        None => println!("No default Java environment set"),
                    }
                }
            }
        }
        Ok(())
    }

    /// 处理 Maven 命令
    async fn handle_maven_command(&mut self, action: MavenCommands) -> Result<(), String> {
        use crate::environments::maven::{MavenInstaller, MirrorDirectoryDiscovery};
        use crate::infrastructure::tool_protocol::VersionDiscovery;
        match action {
            MavenCommands::List { json } => {
                let output = self
                    .switcher
                    .list_environments_with_default(
                        EnvironmentType::Maven,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            MavenCommands::Use { name, shell, json } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => None,
                };
                let result = self
                    .switcher
                    .switch_environment(
                        EnvironmentType::Maven,
                        &name,
                        shell_type,
                        Some("Manual switch via command".to_string()),
                    )
                    .await?;
                if json {
                    let output = FORMATTER.format_switch_result(&result, OutputFormat::Json)?;
                    print!("{output}");
                } else if result.success {
                    // 非 JSON 直接输出切换脚本(供 shell source)
                    if !result.script.is_empty() {
                        print!("{}", result.script);
                    } else {
                        println!("Switched to Maven environment: {name}");
                    }
                } else {
                    eprintln!(
                        "Failed to switch Maven environment: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                    return Err("Environment switch failed".to_string());
                }
            }
            MavenCommands::Install { version, auto_switch } => {
                let mut config = crate::infrastructure::config::Config::load()
                    .map_err(|e| format!("Failed to load config: {e}"))?;
                MavenInstaller::install_maven(&version, &mut config, auto_switch).await?;
            }
            MavenCommands::Uninstall { name } => {
                let mut config = crate::infrastructure::config::Config::load()
                    .map_err(|e| format!("Failed to load config: {e}"))?;
                MavenInstaller::uninstall_maven(&name, &mut config)?;
            }
            MavenCommands::Refresh => {
                let discovery = MirrorDirectoryDiscovery::new();
                discovery.refresh().await.map_err(|e| format!("{e:?}"))?;
                println!("Maven version cache refreshed.");
            }
            MavenCommands::LsRemote { version } => {
                let discovery = MirrorDirectoryDiscovery::new();
                let versions = discovery.list().await.map_err(|e| format!("{e:?}"))?;
                println!("Available Maven versions:");
                let mut shown = 0;
                for v in versions.iter().take(30) {
                    if let Some(f) = version.as_deref() {
                        if !v.version.starts_with(f) {
                            continue;
                        }
                    }
                    println!("  {}", v.version);
                    shown += 1;
                }
                println!("({shown} versions shown)");
            }
            MavenCommands::Current { json } => {
                let output = self
                    .switcher
                    .get_current_environment(
                        EnvironmentType::Maven,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            MavenCommands::Default { name, unset, shell, json } => {
                if unset {
                    let output = self
                        .switcher
                        .clear_default_environment(EnvironmentType::Maven)
                        .await?;
                    print!("{output}");
                } else if let Some(env_name) = name {
                    let output = self
                        .switcher
                        .set_default_environment(EnvironmentType::Maven, &env_name)
                        .await?;
                    print!("{output}");
                } else {
                    match self
                        .switcher
                        .get_default_environment(EnvironmentType::Maven)
                        .await?
                    {
                        Some(env_name) => {
                            if let Some(shell) = shell {
                                let shell_type = parse_shell_type(&shell)?;
                                let result = self
                                    .switcher
                                    .switch_environment(
                                        EnvironmentType::Maven,
                                        &env_name,
                                        Some(shell_type),
                                        Some("Switch to default environment".to_string()),
                                    )
                                    .await?;
                                if json {
                                    let output =
                                        FORMATTER.format_switch_result(&result, OutputFormat::Json)?;
                                    print!("{output}");
                                } else if result.success && !result.script.is_empty() {
                                    print!("{}", result.script);
                                }
                            } else {
                                println!("Default Maven environment: {env_name}");
                            }
                        }
                        None => println!("No default Maven environment set"),
                    }
                }
            }
        }
        Ok(())
    }

    /// 处理 CC 命令
    async fn handle_cc_command(&mut self, action: CcCommands) -> Result<(), String> {
        match action {
            CcCommands::List { json } => {
                let output = self
                    .switcher
                    .list_environments_with_default(
                        EnvironmentType::Cc,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            CcCommands::Use { name, shell, json } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(crate::infrastructure::shell::platform::detect_shell()),
                };
                let result = self
                    .switcher
                    .switch_environment(
                        EnvironmentType::Cc,
                        &name,
                        shell_type,
                        Some("Manual switch via command".to_string()),
                    )
                    .await?;

                // 对于 JSON 输出，格式化显示结果
                if json {
                    let output = FORMATTER.format_switch_result(&result, OutputFormat::Json)?;
                    print!("{output}");
                } else if result.success {
                    // 对于非 JSON 输出，直接输出切换脚本（类似 fnm 的行为）
                    if !result.script.is_empty() {
                        print!("{}", result.script);
                    } else {
                        // 如果没有脚本，显示成功消息
                        println!("Switched to CC environment: {name}");
                    }
                } else {
                    // 如果切换失败，显示错误信息
                    eprintln!(
                        "Failed to switch CC environment: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                    return Err("Environment switch failed".to_string());
                }
            }
            CcCommands::Default {
                name,
                unset,
                shell,
                json,
            } => {
                if unset {
                    let output = self
                        .switcher
                        .clear_default_environment(EnvironmentType::Cc)
                        .await?;
                    print!("{output}");
                } else if let Some(env_name) = name {
                    let output = self
                        .switcher
                        .set_default_environment(EnvironmentType::Cc, &env_name)
                        .await?;
                    print!("{output}");
                } else {
                    match self
                        .switcher
                        .get_default_environment(EnvironmentType::Cc)
                        .await?
                    {
                        Some(env_name) => {
                            if let Some(shell_name) = shell {
                                match parse_shell_type(&shell_name) {
                                    Ok(shell_type) => {
                                        let result = self
                                            .switcher
                                            .switch_environment(
                                                EnvironmentType::Cc,
                                                &env_name,
                                                Some(shell_type),
                                                Some("Switch to default environment".to_string()),
                                            )
                                            .await?;
                                        let output = FORMATTER.format_switch_result(
                                            &result,
                                            if json {
                                                OutputFormat::Json
                                            } else {
                                                OutputFormat::Text
                                            },
                                        )?;
                                        print!("{output}");
                                    }
                                    Err(e) => return Err(e),
                                }
                            } else {
                                println!("Default CC environment: {env_name}");
                            }
                        }
                        None => println!("No default CC environment set"),
                    }
                }
            }
            CcCommands::Current { json } => {
                let output = self
                    .switcher
                    .get_current_environment(
                        EnvironmentType::Cc,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            CcCommands::Add {
                name,
                provider,
                api_key,
                base_url,
                model,
                description,
            } => {
                use crate::infrastructure::config::{CcEnvironment, Config};

                let mut config = Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
                let env = CcEnvironment {
                    name: name.clone(),
                    provider: provider.clone(),
                    api_key: api_key.unwrap_or_default(),
                    base_url: base_url.unwrap_or_default(),
                    sonnet_model: model.unwrap_or_else(|| "claude-sonnet-4-5".to_string()),
                    opus_model: None,
                    haiku_model: None,
                    description: description.unwrap_or_default(),
                };
                // Check duplicate
                if config.cc_environments.iter().any(|e| e.name == env.name) {
                    return Err(format!("CC environment '{}' already exists", env.name));
                }
                config.cc_environments.push(env);
                config.save()?;
                println!("CC environment '{name}' added ({provider})");
            }
            CcCommands::Remove { name } => {
                let output = self
                    .switcher
                    .remove_environment(EnvironmentType::Cc, &name)
                    .await?;
                print!("{output}");
            }
        }
        Ok(())
    }


    /// 处理配置命令
    async fn handle_config_command(&mut self, action: ConfigCommands) -> Result<(), String> {
        match action {
            ConfigCommands::Sync => {
                use crate::infrastructure::config::Config;
                let updated = Config::sync()?;
                if updated {
                    println!("Configuration synced");
                } else {
                    println!("Configuration is up to date");
                }
            }
        }
        Ok(())
    }

    /// 处理 Java 远程查询（简化版本）
    async fn handle_java_ls_remote(
        &self,
        java_version: Option<u32>,
        _repository: Option<String>,
    ) -> Result<String, String> {
        use crate::environments::java::installer::JavaInstaller;

        println!("Querying available Java versions...");

        // 暂时使用旧的实现，确保基本功能可用
        match JavaInstaller::list_installable_versions().await {
            Ok(versions) => {
                let mut output = String::new();
                output.push_str("Available Java versions:\n\n");

                if let Some(major) = java_version {
                    let filtered_versions: Vec<String> = versions
                        .into_iter()
                        .filter(|v| v.contains(&major.to_string()))
                        .collect();

                    if filtered_versions.is_empty() {
                        output.push_str(&format!("No Java {major} versions found\n"));
                    } else {
                        output.push_str(&format!("Available Java {major} versions:\n"));
                        for version in filtered_versions {
                            output.push_str(&format!("  {version}\n"));
                        }
                    }
                } else {
                    output.push_str("All available versions:\n");
                    for version in versions {
                        output.push_str(&format!("  {version}\n"));
                    }
                }

                output.push_str("\nUsage:\n");
                output.push_str("  fnva java install 21        # Install Java 21\n");
                output.push_str("  fnva java install lts        # Install latest LTS\n");
                output.push_str("  fnva java install latest     # Install latest version\n");

                Ok(output)
            }
            Err(e) => Err(format!("Failed to query versions: {e}")),
        }
    }

    /// 处理历史命令
    async fn handle_history_command(
        &self,
        env_type: Option<String>,
        limit: usize,
        _json: bool,
    ) -> Result<(), String> {
        let env_type = env_type.map(|t| parse_environment_type(&t)).transpose()?;
        let output = self.switcher.get_switch_history(env_type, limit).await?;
        print!("{output}");
        Ok(())
    }
}
