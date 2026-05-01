mod config;

use config::CliConfig;

pub enum Command {
    Serve,
    Check,
}

pub fn run(command: Command, key: &str) -> bool {
    let config = CliConfig {
        token: std::env::var(key).unwrap_or_default(),
    };
    matches!(command, Command::Serve | Command::Check) || !config.token.is_empty()
}

fn main() {
    let _ = config::config_version();
    let _ = run(Command::Check, "TOKEN");
}
