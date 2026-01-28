use clap::{Parser, Subcommand};

mod config;
mod tmux;

#[derive(Parser, Debug)]
#[command(name = "mux", version, about = "tmux orchestrator")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create or open a config
    #[command(alias = "o")]
    Open { config_name: String },

    #[command(alias = "s")]
    Start { config_name: String },

    #[command(alias = "r")]
    Remove { config_name: String },

    #[command(alias = "ks")]
    KillSession,

    #[command(alias = "kw")]
    KillWindow,

    #[command(alias = "kp")]
    KillPane,
}

fn start(config_name: &str) {}

fn remove(config_name: &str) {
    config::remove_config(config_name);
    println!("removed config: {config_name}");
}

fn main() {
    let cli = Cli::parse();
    match &cli.cmd {
        Command::Open { config_name } => config::open_or_create_config(config_name),
        Command::Start { config_name } => start(config_name),
        Command::Remove { config_name } => remove(config_name),
        Command::KillSession => tmux::Tmux::kill_session(),
        Command::KillWindow => tmux::Tmux::kill_window(),
        Command::KillPane => tmux::Tmux::kill_pane(),
    }
}
