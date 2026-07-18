use clap::FromArgMatches;
use fnva::cli::print;
use fnva::cli::{Cli, CommandHandler};
use fnva::error::AppError;
use std::process;

#[tokio::main]
async fn main() {
    let cli =
        Cli::from_arg_matches(&Cli::command().get_matches()).expect("Failed to parse arguments");

    let mut handler = match CommandHandler::new() {
        Ok(handler) => handler,
        Err(e) => {
            report_error(&e);
            process::exit(1);
        }
    };

    if let Err(e) = handler.handle_command(cli.command).await {
        report_error(&e);
        process::exit(1);
    }
}

/// 按错误类型打印用户友好的错误信息 + 一条修复建议。
fn report_error(e: &AppError) {
    print::failure("Command failed", Some(&e.to_string()));
    let hint: &str = match e.root_cause() {
        AppError::NotFound { .. } => "Run `fnva <type> list` to see available environment names.",
        AppError::Network { .. } => {
            "Check mirror URLs / proxy in ~/.fnva/config.toml, or your network connection."
        }
        AppError::Permission { .. } => "Check ownership and permissions of ~/.fnva.",
        AppError::Config { .. } => "Inspect / fix ~/.fnva/config.toml (or run `fnva config sync`).",
        _ => return,
    };
    eprintln!("  {}", hint);
}
