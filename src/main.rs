use clap::{Parser, Subcommand};

mod config;
mod tmux;
mod util;

#[derive(Parser, Debug)]
#[command(name = "mux", version, about = "tmux orchestrator")]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Create or open a config
    #[command(alias = "o", aliases=["create", "c", "new", "n"])]
    Open { config_name: String },

    #[command(alias = "s")]
    Start { config_name: String },

    #[command(alias = "r", aliases=["rm", "delete", "d"])]
    Remove { config_name: String },

    #[command(alias = "ks")]
    KillSession,

    #[command(alias = "kw")]
    KillWindow,

    #[command(alias = "kp")]
    KillPane,
}

fn start(config_name: &str) {
    let cfg = match config::parse_config(config_name) {
        Ok(cfg) => cfg,
        Err(e) => panic!("failed to load config: {e}"),
    };

    let t = tmux::Tmux::new(&cfg.name, &cfg.root);
    if t.is_session_exist() {
        t.attach_or_switch();
        return;
    }

    t.start_in_background();

    let windows = &cfg.windows;
    for window in windows {
        match window {
            config::Window::WindowWithPanes(mp) => {
                for (window_name, cmd) in mp {
                    match cmd {
                        config::CmdPanes::Panes(panes) => {
                            t.add_window(&window_name, &panes.root);
                            for pane in &panes.panes {
                                match pane {
                                    config::Pane::PaneWithCommands(mp) => {
                                        assert!(mp.len() == 1);
                                        let (pane_name, cmds) = mp.iter().next().unwrap();
                                        let pane_id = t.split_window(
                                            &format!("{}:{}", &cfg.name, &window_name),
                                            Some(pane_name),
                                            &panes.layout.as_ref(),
                                        );
                                        for cmd in cmds {
                                            t.send_cmd(
                                                &format!(
                                                    "{}:{}.{}",
                                                    &cfg.name, &window_name, &pane_id
                                                ),
                                                cmd,
                                            )
                                        }
                                    }
                                    config::Pane::Command(cmd) => {
                                        let pane_id = t.split_window(
                                            &format!("{}:{}", &cfg.name, &window_name),
                                            None,
                                            &panes.layout.as_ref(),
                                        );
                                        t.send_cmd(
                                            &format!("{}:{}.{}", &cfg.name, &window_name, &pane_id),
                                            cmd,
                                        );
                                    }
                                }
                            }
                        }
                        config::CmdPanes::Command(cmd) => {
                            t.add_window(&window_name, &None);
                            t.send_cmd(&format!("{}:{}", &cfg.name, &window_name), cmd);
                        }
                    }
                }
            }
            config::Window::WindowName(name) => {
                t.add_window(&name, &None);
            }
        }
    }

    t.attach_or_switch();
}

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
