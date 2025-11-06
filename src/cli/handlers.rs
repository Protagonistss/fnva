use crate::cli::commands::*;
use crate::cli::output::{OutputFormat, FORMATTER};
use crate::core::switcher::EnvironmentSwitcher;
use crate::core::environment_manager::{EnvironmentType, EnvironmentManagerFactory};
use crate::infrastructure::shell::platform::detect_shell;
use std::sync::{Arc, Mutex};

/// 命令处理器
pub struct CommandHandler {
    switcher: EnvironmentSwitcher,
}

impl CommandHandler {
    /// 创建新的命令处理器
    pub fn new() -> Result<Self, String> {
        let mut switcher = EnvironmentSwitcher::new()?;

        // 注册 Java 环境管理器
        let java_manager = crate::environments::java::JavaEnvironmentManager::new();
        switcher.register_manager(Arc::new(Mutex::new(java_manager)));

        // 注册 LLM 环境管理器
        let llm_manager = crate::environments::llm::LlmEnvironmentManager::new();
        switcher.register_manager(Arc::new(Mutex::new(llm_manager)));

        // 注册 CC 环境管理器
        let cc_manager = crate::environments::cc::CcEnvironmentManager::new();
        switcher.register_manager(Arc::new(Mutex::new(cc_manager)));

        Ok(Self { switcher })
    }

    /// 处理命令
    pub async fn handle_command(&mut self, command: Commands) -> Result<(), String> {
        match command {
            Commands::Java { action } => self.handle_java_command(action).await,
            Commands::Llm { action } => self.handle_llm_command(action).await,
            Commands::Cc { action } => self.handle_cc_command(action).await,
            Commands::Env { action } => self.handle_env_command(action).await,
            Commands::NetworkTest => self.handle_network_test().await,
            Commands::History { env_type, limit, json } => {
                self.handle_history_command(env_type, limit, json).await
            }
        }
    }

    /// 处理 Java 命令
    async fn handle_java_command(&mut self, action: JavaCommands) -> Result<(), String> {
        match action {
            JavaCommands::List { json } => {
                let output = self.switcher.list_environments_with_default(
                    EnvironmentType::Java,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            JavaCommands::Use { name, shell, json } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(crate::infrastructure::shell::platform::detect_shell()),
                };

                let result = self.switcher.switch_environment(
                    EnvironmentType::Java,
                    &name,
                    shell_type,
                    Some("Manual switch via command".to_string())
                ).await?;

                // 对于 JSON 输出，格式化显示结果
                if json {
                    let output = FORMATTER.format_switch_result(&result, OutputFormat::Json)?;
                    print!("{}", output);
                } else if result.success {
                    // 对于非 JSON 输出，直接输出切换脚本（类似 fnm 的行为）
                    if !result.script.is_empty() {
                        print!("{}", result.script);
                    } else {
                        // 如果没有脚本，显示成功消息
                        println!("Switched to Java environment: {}", name);
                    }
                } else {
                    // 如果切换失败，显示错误信息
                    eprintln!("Failed to switch Java environment: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string()));
                    return Err("Environment switch failed".to_string());
                }
            }
            JavaCommands::Current { json } => {
                let output = self.switcher.get_current_environment(
                    EnvironmentType::Java,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            JavaCommands::Scan => {
                let output = self.switcher.scan_environments(EnvironmentType::Java).await?;
                print!("{}", output);
            }
            JavaCommands::Add { name, home, description } => {
                let config_value = serde_json::json!({
                    "java_home": home
                });
                let output = self.switcher.add_environment(EnvironmentType::Java, &name, config_value).await?;
                print!("{}", output);
            }
            JavaCommands::Remove { name } => {
                let output = self.switcher.remove_environment(EnvironmentType::Java, &name).await?;
                print!("{}", output);
            }
            JavaCommands::Default { name, unset, shell, json } => {
                if unset {
                    // 清除默认环境
                    let output = self.switcher.clear_default_environment(EnvironmentType::Java).await?;
                    print!("{}", output);
                } else if let Some(env_name) = name {
                    // 设置默认环境
                    let output = self.switcher.set_default_environment(EnvironmentType::Java, &env_name).await?;
                    print!("{}", output);
                } else {
                    // 显示当前默认环境
                    match self.switcher.get_default_environment(EnvironmentType::Java).await? {
                        Some(env_name) => {
                            if let Some(shell) = shell {
                                match parse_shell_type(&shell) {
                                    Ok(shell_type) => {
                                        let result = self.switcher.switch_environment(
                                            EnvironmentType::Java,
                                            &env_name,
                                            Some(shell_type),
                                            Some("Switch to default environment".to_string())
                                        ).await?;
                                        let output = FORMATTER.format_switch_result(&result,
                                            if json { OutputFormat::Json } else { OutputFormat::Text })?;
                                        print!("{}", output);
                                    }
                                    Err(e) => return Err(e),
                                }
                            } else {
                                println!("Default Java environment: {}", env_name);
                            }
                        }
                        None => println!("No default Java environment set"),
                    }
                }
            }
            // 其他 Java 命令...
            _ => {
                return Err("Java command not yet implemented in new architecture".to_string());
            }
        }
        Ok(())
    }

    /// 处理 LLM 命令
    async fn handle_llm_command(&mut self, action: LlmCommands) -> Result<(), String> {
        match action {
            LlmCommands::List { json } => {
                let output = self.switcher.list_environments(
                    EnvironmentType::Llm,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            LlmCommands::Use { name, shell, json } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => None,
                };
                let result = self.switcher.switch_environment(
                    EnvironmentType::Llm,
                    &name,
                    shell_type,
                    Some("Manual switch via command".to_string())
                ).await?;

                let output = FORMATTER.format_switch_result(&result,
                    if json { OutputFormat::Json } else { OutputFormat::Text });
                print!("{}", output?);
            }
            LlmCommands::Current { json } => {
                let output = self.switcher.get_current_environment(
                    EnvironmentType::Llm,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            // 其他 LLM 命令...
            _ => {
                return Err("LLM command not yet implemented in new architecture".to_string());
            }
        }
        Ok(())
    }

    /// 处理 CC 命令
    async fn handle_cc_command(&mut self, action: CcCommands) -> Result<(), String> {
        match action {
            CcCommands::List { json } => {
                let output = self.switcher.list_environments(
                    EnvironmentType::Cc,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            CcCommands::Use { name, shell, json } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(crate::infrastructure::shell::platform::detect_shell()),
                };
                let result = self.switcher.switch_environment(
                    EnvironmentType::Cc,
                    &name,
                    shell_type,
                    Some("Manual switch via command".to_string())
                ).await?;

                // 对于 JSON 输出，格式化显示结果
                if json {
                    let output = FORMATTER.format_switch_result(&result, OutputFormat::Json)?;
                    print!("{}", output);
                } else if result.success {
                    // 对于非 JSON 输出，直接输出切换脚本（类似 fnm 的行为）
                    if !result.script.is_empty() {
                        print!("{}", result.script);
                    } else {
                        // 如果没有脚本，显示成功消息
                        println!("Switched to CC environment: {}", name);
                    }
                } else {
                    // 如果切换失败，显示错误信息
                    eprintln!("Failed to switch CC environment: {}",
                        result.error.unwrap_or_else(|| "Unknown error".to_string()));
                    return Err("Environment switch failed".to_string());
                }
            }
            CcCommands::Current { json } => {
                let output = self.switcher.get_current_environment(
                    EnvironmentType::Cc,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            // 其他 CC 命令...
            _ => {
                return Err("CC command not yet implemented in new architecture".to_string());
            }
        }
        Ok(())
    }

    /// 处理环境管理命令
    async fn handle_env_command(&mut self, action: EnvCommands) -> Result<(), String> {
        match action {
            EnvCommands::GenerateEnv { shell, use_on_cd } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(detect_shell()),
                };

                // 生成类似 fnm env 的环境变量设置脚本
                let script = match shell_type.unwrap() {
                    crate::infrastructure::shell::ShellType::PowerShell => {
                        r#"
# fnva environment setup
$env:FNVA_SHELL_INTEGRATION = $true

# Auto-load default Java environment (like fnm)
try {
    $defaultEnvRaw = & fnva.exe java default 2>$null
    if ($LASTEXITCODE -eq 0 -and $defaultEnvRaw -and $defaultEnvRaw -notmatch "No default") {
        # Extract environment name from output like "Default Java environment: jdk21.0.6"
        $defaultEnv = ($defaultEnvRaw -split ':')[-1].Trim()
        Write-Host "Loading default Java environment: $defaultEnv" -ForegroundColor Cyan
        $switchScript = & fnva.exe java use $defaultEnv --shell powershell 2>$null
        if ($LASTEXITCODE -eq 0 -and $switchScript) {
            if ($switchScript -is [array]) {
                $switchScript = $switchScript -join "`r`n"
            }
            Invoke-Expression $switchScript
        }
    }
} catch {
    # Ignore errors during default loading
}

function fnva {
    param(
        [Parameter(ValueFromRemainingArguments=$true)]
        [string[]]$Args
    )

    if ($Args.Count -ge 3 -and $Args[1] -eq "use") {
        $envType = $Args[0]
        $envName = $Args[2]
        $output = & fnva.exe $Args[0] use $Args[2] --shell powershell 2>$null
        if ($output -is [array]) {
            $script = $output -join "`r`n"
        } else {
            $script = $output
        }

        # Check if script contains relevant environment variables
        $isValidScript = $false
        if ($envType -eq "java" -and $script -match "JAVA_HOME") {
            $isValidScript = $true
        } elseif ($envType -eq "cc" -and ($script -match "ANTHROPIC_AUTH_TOKEN" -or $script -match "ANTHROPIC_BASE_URL")) {
            $isValidScript = $true
        }

        if ($LASTEXITCODE -eq 0 -and $isValidScript) {
            try {
                Invoke-Expression $script
                Write-Host "Switched to $envType`: $envName" -ForegroundColor Green
            } catch {
                Write-Error "Failed to execute switch script: $($_.Exception.Message)"
            }
        } else {
            Write-Output $output
        }
    } else {
        & fnva.exe $Args
    }
}
"#.to_string()
                    }
                    _ => {
                        "# fnva environment setup for other shells\nexport FNVA_SHELL_INTEGRATION=true\n".to_string()
                    }
                };

                print!("{}", script);
            }
                        EnvCommands::Switch { env_type, name, shell, reason, json } => {
                let env_type = parse_environment_type(&env_type)?;
                let shell_type = match shell {
            Some(s) => Some(parse_shell_type(&s)?),
            None => None,
        };
                let result = self.switcher.switch_environment(
                    env_type,
                    &name,
                    shell_type,
                    reason
                ).await?;

                let output = FORMATTER.format_switch_result(&result,
                    if json { OutputFormat::Json } else { OutputFormat::Text });
                print!("{}", output?);
            }
            EnvCommands::List { env_type, json } => {
                let env_type = match env_type {
            Some(t) => parse_environment_type(&t)?,
            None => EnvironmentType::Java,
        };
                let output = self.switcher.list_environments(
                    env_type,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            EnvCommands::Current { env_type, json } => {
                let env_type = match env_type {
            Some(t) => parse_environment_type(&t)?,
            None => EnvironmentType::Java,
        };
                let output = self.switcher.get_current_environment(
                    env_type,
                    if json { OutputFormat::Json } else { OutputFormat::Text }
                ).await?;
                print!("{}", output);
            }
            EnvCommands::ShellIntegration { shell } => {
                let shell_type = match shell {
                    Some(s) => Some(parse_shell_type(&s)?),
                    None => Some(crate::infrastructure::shell::platform::detect_shell()),
                };
                let output = self.switcher.generate_shell_integration(shell_type.unwrap()).await?;
                print!("{}", output);
            }
            // 其他环境命令...
            _ => {
                return Err("Environment command not yet implemented in new architecture".to_string());
            }
        }
        Ok(())
    }

    /// 处理网络测试命令
    async fn handle_network_test(&self) -> Result<(), String> {
        // TODO: 实现网络测试
        println!("Network test not yet implemented in new architecture");
        Ok(())
    }

    /// 处理历史命令
    async fn handle_history_command(&self, env_type: Option<String>, limit: usize, _json: bool) -> Result<(), String> {
        let env_type = env_type.map(|t| parse_environment_type(&t)).transpose()?;
        let output = self.switcher.get_switch_history(env_type, limit).await?;
        print!("{}", output);
        Ok(())
    }
}