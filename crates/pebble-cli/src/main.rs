mod args;
mod client;
mod daemon;
mod log;

use std::process::ExitCode;

use pebble_core::adapters::config::ConfigPaths;

#[tokio::main]
async fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty()
        || raw
            .iter()
            .any(|a| a == "-h" || a == "--help" || a == "help")
    {
        println!("{}", args::USAGE);
        return ExitCode::SUCCESS;
    }
    let cli = match args::parse(raw) {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{e}\n\n{}", args::USAGE);
            return ExitCode::from(2);
        }
    };
    let config = cli
        .config_dir
        .map(ConfigPaths::new)
        .unwrap_or_else(ConfigPaths::user_default);
    let result = match cli.command {
        args::Command::Daemon(options) => daemon::run(config, options)
            .await
            .map_err(|e| e.to_string()),
        args::Command::Request(request) => {
            client::run(&config.control_socket(), request, cli.json).await
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            if !message.is_empty() {
                eprintln!("오류: {message}");
            }
            ExitCode::FAILURE
        }
    }
}
