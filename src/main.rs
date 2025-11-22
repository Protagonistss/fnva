use clap::Parser;
use fnva::cli::{Cli, CommandHandler};
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut handler = match CommandHandler::new() {
        Ok(handler) => handler,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = handler.handle_command(cli.command).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
