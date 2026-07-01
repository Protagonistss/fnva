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
                        crate::cli::print::success(&format!("{name}  [java]"));
                    }
                } else {
                    // 如果切换失败，显示错误信息
                    crate::cli::print::failure(
                        "Failed to switch java environment",
                        Some(result.error.as_deref().unwrap_or("Unknown error")),
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
            JavaCommands::LsRemote { version } => {
                let output = self.handle_java_ls_remote(version).await?;
                print!("{output}");
            }
            JavaCommands::Refresh => {
                use crate::environments::java::downloader::JavaDownloader;
                use crate::infrastructure::config::Config;
                let config = Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
                let downloader = JavaDownloader::new(config.mirrors.java);
                downloader.refresh().await.map_err(|e| format!("{e:?}"))?;
                crate::cli::print::success("Java version cache refreshed");
            }
            JavaCommands::Install {
                version,
                auto_switch,
            } => {
                use crate::environments::java::installer::JavaInstaller;
                use crate::infrastructure::config::Config;

                let mut config =
                    Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
                match JavaInstaller::install_java(&version, &mut config, auto_switch).await {
                    Ok(java_home) => {
                        crate::cli::print::success(&format!("java {version} installed"));
                        crate::cli::print::detail("Path", &java_home);
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

                let mut config =
                    Config::load().map_err(|e| format!("Failed to load config: {e}"))?;
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
                                crate::cli::print::success(&format!(
                                    "{env_name}  [java] (default)"
                                ));
                            }
                        }
                        None => crate::cli::print::warn("No default java environment set"),
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
                        crate::cli::print::success(&format!("{name}  [maven]"));
                    }
                } else {
                    crate::cli::print::failure(
                        "Failed to switch maven environment",
                        Some(result.error.as_deref().unwrap_or("Unknown error")),
                    );
                    return Err("Environment switch failed".to_string());
                }
            }
            MavenCommands::Install {
                version,
                auto_switch,
            } => {
                let mut config = crate::infrastructure::config::Config::load()
                    .map_err(|e| format!("Failed to load config: {e}"))?;
                MavenInstaller::install_maven(&version, &mut config, auto_switch).await?;
            }
            MavenCommands::Scan => {
                let output = self
                    .switcher
                    .scan_environments(EnvironmentType::Maven)
                    .await?;
                print!("{output}");
            }
            MavenCommands::Add {
                name,
                home,
                description: _,
                maven_opts,
                local_repo,
                settings,
            } => {
                let mut config_value = serde_json::json!({ "maven_home": home });
                if let Some(v) = maven_opts {
                    config_value["maven_opts"] = serde_json::json!(v);
                }
                if let Some(v) = local_repo {
                    config_value["local_repo"] = serde_json::json!(v);
                }
                if let Some(v) = settings {
                    config_value["settings_file"] = serde_json::json!(v);
                }
                let output = self
                    .switcher
                    .add_environment(EnvironmentType::Maven, &name, config_value)
                    .await?;
                print!("{output}");
            }
            MavenCommands::Remove { name } => {
                let output = self
                    .switcher
                    .remove_environment(EnvironmentType::Maven, &name)
                    .await?;
                print!("{output}");
            }
            MavenCommands::Uninstall { name } => {
                let mut config = crate::infrastructure::config::Config::load()
                    .map_err(|e| format!("Failed to load config: {e}"))?;
                MavenInstaller::uninstall_maven(&name, &mut config)?;
            }
            MavenCommands::Refresh => {
                let discovery = MirrorDirectoryDiscovery::new();
                discovery.refresh().await.map_err(|e| format!("{e:?}"))?;
                crate::cli::print::success("Maven version cache refreshed");
            }
            MavenCommands::LsRemote { version } => {
                let discovery = MirrorDirectoryDiscovery::new();
                let versions = discovery.list().await.map_err(|e| format!("{e:?}"))?;
                crate::cli::print::step("Status", "Available Maven versions:");
                let mut shown = 0;
                for v in versions.iter().take(30) {
                    if let Some(f) = version.as_deref() {
                        if !v.version.starts_with(f) {
                            continue;
                        }
                    }
                    crate::cli::print::step("version", &v.version);
                    shown += 1;
                }
                crate::cli::print::step("Status", &format!("({shown} versions shown)"));
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
            MavenCommands::Default {
                name,
                unset,
                shell,
                json,
            } => {
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
                                    let output = FORMATTER
                                        .format_switch_result(&result, OutputFormat::Json)?;
                                    print!("{output}");
                                } else if result.success && !result.script.is_empty() {
                                    print!("{}", result.script);
                                }
                            } else {
                                crate::cli::print::success(&format!(
                                    "{env_name}  [maven] (default)"
                                ));
                            }
                        }
                        None => crate::cli::print::warn("No default maven environment set"),
                    }
                }
            }
            MavenCommands::Set {
                name,
                maven_opts,
                local_repo,
                settings,
                unset_maven_opts,
                unset_local_repo,
                unset_settings,
            } => {
                use crate::environments::maven::MavenEnvironmentManager;
                let mut manager = MavenEnvironmentManager::new();

                // Some(Some(value)) = 设置; Some(None) = 清除; None = 不变
                let opts_arg = if unset_maven_opts {
                    Some(None)
                } else {
                    maven_opts.map(Some)
                };
                let repo_arg = if unset_local_repo {
                    Some(None)
                } else {
                    local_repo.map(Some)
                };
                let settings_arg = if unset_settings {
                    Some(None)
                } else {
                    settings.map(Some)
                };

                manager.set_env_vars(&name, opts_arg, repo_arg, settings_arg)?;
                crate::cli::print::success(&format!("Updated Maven environment: {name}"));
            }
            MavenCommands::Show { name } => {
                use crate::environments::maven::MavenEnvironmentManager;
                let manager = MavenEnvironmentManager::new();
                let info = manager.show_env(&name)?;
                println!("{info}");
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
            CcCommands::Scan => {
                let output = self.switcher.scan_environments(EnvironmentType::Cc).await?;
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
                        crate::cli::print::success(&format!("{name}  [cc]"));
                    }
                } else {
                    // 如果切换失败，显示错误信息
                    crate::cli::print::failure(
                        "Failed to switch cc environment",
                        Some(result.error.as_deref().unwrap_or("Unknown error")),
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
                                crate::cli::print::success(&format!("{env_name}  [cc] (default)"));
                            }
                        }
                        None => crate::cli::print::warn("No default cc environment set"),
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
                let base_url_val = base_url.unwrap_or_default();
                if base_url_val.is_empty() {
                    return Err("Missing required argument: --base-url <URL>\n\
                         Example: fnva cc add --name my-cc --provider anthropic \
                         --base-url https://api.anthropic.com --api-key ${ANTHROPIC_API_KEY}"
                        .to_string());
                }
                let mut json = serde_json::json!({
                    "provider": provider,
                    "base_url": base_url_val,
                });
                if let Some(k) = api_key {
                    json["api_key"] = serde_json::Value::String(k);
                }
                if let Some(m) = model {
                    json["sonnet_model"] = serde_json::Value::String(m);
                }
                if let Some(d) = description {
                    json["description"] = serde_json::Value::String(d);
                }
                let output = self
                    .switcher
                    .add_environment(EnvironmentType::Cc, &name, json)
                    .await?;
                print!("{output}");
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
                    crate::cli::print::success("Configuration synced");
                } else {
                    crate::cli::print::success("Configuration is up to date");
                }
            }
        }
        Ok(())
    }

    /// Handle Java remote version listing.
    async fn handle_java_ls_remote(&self, version: Option<u32>) -> Result<String, String> {
        use crate::environments::java::installer::JavaInstaller;

        crate::cli::print::action("Querying available Java versions...");

        match JavaInstaller::list_installable_versions().await {
            Ok(versions) => {
                let mut output = String::new();

                if let Some(major) = version {
                    let filtered: Vec<String> = versions
                        .into_iter()
                        .filter(|v| v.contains(&major.to_string()))
                        .collect();
                    if filtered.is_empty() {
                        output.push_str(&format!("No Java {major} versions found\n"));
                    } else {
                        output.push_str(&format!("Available Java {major} versions:\n"));
                        for v in filtered {
                            output.push_str(&format!("  {v}\n"));
                        }
                    }
                } else {
                    output.push_str("Available Java versions:\n");
                    for v in versions {
                        output.push_str(&format!("  {v}\n"));
                    }
                }

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
