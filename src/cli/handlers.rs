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
        let mut switcher = EnvironmentSwitcher::new().map_err(|e| e.to_string())?;

        // 注册 Java 环境管理器
        let java_manager = crate::environments::java::JavaEnvironmentManager::new();
        switcher
            .register_manager(Arc::new(Mutex::new(java_manager)))
            .map_err(|e| e.to_string())?;

        // 注册 LLM 环境管理器
        let llm_manager = crate::environments::llm::LlmEnvironmentManager::new();
        switcher
            .register_manager(Arc::new(Mutex::new(llm_manager)))
            .map_err(|e| e.to_string())?;

        // 注册 CC 环境管理器
        let cc_manager = crate::environments::cc::CcEnvironmentManager::new();
        switcher
            .register_manager(Arc::new(Mutex::new(cc_manager)))
            .map_err(|e| e.to_string())?;

        Ok(Self { switcher })
    }

    /// 处理命令
    pub async fn handle_command(&mut self, command: Commands) -> Result<(), String> {
        match command {
            Commands::Java { action } => self.handle_java_command(action).await,
            Commands::Llm { action } => self.handle_llm_command(action).await,
            Commands::Cc { action } => self.handle_cc_command(action).await,
            Commands::Env { action } => self.handle_env_command(action).await,
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
                java_version,
                maven_artifact: _,
                search: _,
                repository,
                limit: _,
            } => {
                if query_type == "java" {
                    // 使用新的版本管理器查询 Java 版本
                    let output = self.handle_java_ls_remote(java_version, repository).await?;
                    print!("{output}");
                } else {
                    return Err(format!("查询类型 '{query_type}' 尚不支持"));
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
                        println!("[OK] Java {version} installed successfully");
                        println!("     Path: {java_home}");
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

    /// 处理 LLM 命令
    async fn handle_llm_command(&mut self, action: LlmCommands) -> Result<(), String> {
        match action {
            LlmCommands::List { json } => {
                let output = self
                    .switcher
                    .list_environments(
                        EnvironmentType::Llm,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            LlmCommands::Use { name, shell, json } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => None,
                };
                let result = self
                    .switcher
                    .switch_environment(
                        EnvironmentType::Llm,
                        &name,
                        shell_type,
                        Some("Manual switch via command".to_string()),
                    )
                    .await?;

                let output = FORMATTER.format_switch_result(
                    &result,
                    if json {
                        OutputFormat::Json
                    } else {
                        OutputFormat::Text
                    },
                );
                print!("{}", output?);
            }
            LlmCommands::Current { json } => {
                let output = self
                    .switcher
                    .get_current_environment(
                        EnvironmentType::Llm,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
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
                use crate::infrastructure::config::{Config, LlmEnvironment};

                let mut config = Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
                let env = LlmEnvironment {
                    name: name.clone(),
                    provider: provider.clone(),
                    api_key: api_key.unwrap_or_default(),
                    base_url: base_url.unwrap_or_default(),
                    model: model.unwrap_or_default(),
                    temperature,
                    max_tokens,
                    description: description.unwrap_or_default(),
                };
                config.add_llm_env(env)?;
                config.save()?;
                println!("LLM environment '{name}' added ({provider})");
            }
            LlmCommands::Remove { name } => {
                let output = self
                    .switcher
                    .remove_environment(EnvironmentType::Llm, &name)
                    .await?;
                print!("{output}");
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

    /// 处理环境管理命令
    async fn handle_env_command(&mut self, action: EnvCommands) -> Result<(), String> {
        match action {
            EnvCommands::GenerateEnv {
                shell,
                use_on_cd: _,
            } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(detect_shell()),
                };

                // 生成完整的环境设置脚本（autoload + wrapper）
                let script = self
                    .switcher
                    .generate_shell_integration(shell_type.unwrap())
                    .await?;
                print!("{script}");
            }
            EnvCommands::Switch {
                env_type,
                name,
                shell,
                reason,
                json,
            } => {
                let env_type = parse_environment_type(&env_type)?;
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => None,
                };
                let result = self
                    .switcher
                    .switch_environment(env_type, &name, shell_type, reason)
                    .await?;

                let output = FORMATTER.format_switch_result(
                    &result,
                    if json {
                        OutputFormat::Json
                    } else {
                        OutputFormat::Text
                    },
                );
                print!("{}", output?);
            }
            EnvCommands::List { env_type, json } => {
                let env_type = match env_type {
                    Some(t) => parse_environment_type(&t)?,
                    None => EnvironmentType::Java,
                };
                let output = self
                    .switcher
                    .list_environments(
                        env_type,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            EnvCommands::Current { env_type, json } => {
                let env_type = match env_type {
                    Some(t) => parse_environment_type(&t)?,
                    None => EnvironmentType::Java,
                };
                let output = self
                    .switcher
                    .get_current_environment(
                        env_type,
                        if json {
                            OutputFormat::Json
                        } else {
                            OutputFormat::Text
                        },
                    )
                    .await?;
                print!("{output}");
            }
            EnvCommands::ShellIntegration { shell } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(crate::infrastructure::shell::platform::detect_shell()),
                };
                let output = self
                    .switcher
                    .generate_shell_integration(shell_type.unwrap())
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
                    println!("配置已同步（补全默认字段，例如清华源）");
                } else {
                    println!("配置已是最新，无需同步");
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
                        output.push_str(&format!("❌ 未找到 Java {major} 的可用版本\n"));
                    } else {
                        output.push_str(&format!("🎯 Java {major} 可用版本:\n"));
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
            Err(e) => Err(format!("查询版本失败: {e}")),
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
