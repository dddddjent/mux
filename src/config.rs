use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::{env, io};

use crate::util::xdg_config_home;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub name: String,
    pub root: String,
    pub windows: Vec<Window>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Window {
    WindowWithPanes(BTreeMap<String, CmdPanes>),
    WindowName(String),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CmdPanes {
    Panes(Panes),
    Command(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Layout {
    #[serde(rename = "even-horizontal")]
    EvenHorizontal,
    #[serde(rename = "even-vertical")]
    EvenVertical,
}

fn default_layout() -> Option<Layout> {
    Some(Layout::EvenHorizontal)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Panes {
    pub panes: Vec<Pane>,
    #[serde(default = "default_layout")]
    pub layout: Option<Layout>,
    #[serde(default)]
    pub root: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Pane {
    PaneWithCommands(BTreeMap<String, Vec<String>>),
    Command(String),
}

impl Config {
    pub fn default() -> Config {
        Config {
            name: "Default".to_string(),
            root: "~/Documents/PlayGround/".to_string(),
            windows: vec![
                Window::WindowWithPanes(BTreeMap::from([(
                    "editor".to_string(),
                    CmdPanes::Panes(Panes {
                        panes: vec![Pane::PaneWithCommands(BTreeMap::from([(
                            "editor".to_string(),
                            vec!["nvim".to_string(), ":RestoreSession".to_string()],
                        )]))],
                        layout: None,
                        root: None,
                    }),
                )])),
                Window::WindowWithPanes(BTreeMap::from([(
                    "misc".to_string(),
                    CmdPanes::Panes(Panes {
                        panes: vec![
                            Pane::Command("clear".to_string()),
                            Pane::Command("clear".to_string()),
                        ],
                        layout: Some(Layout::EvenVertical),
                        root: None,
                    }),
                )])),
            ],
        }
    }
}

pub fn config_dir() -> PathBuf {
    xdg_config_home().join("mux")
}

pub fn parse_config(config_name: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(config_dir().join(format!("{}.yml", config_name)))?;
    let cfg: Config = serde_yml::from_str(&s)?;
    Ok(cfg)
}

pub fn open_or_create_config(config_name: &str) {
    let cfg = parse_config(config_name);
    match cfg {
        Err(e) => {
            if let Some(io_error) = e.downcast_ref::<io::Error>() {
                if io_error.kind() == io::ErrorKind::NotFound {
                    let cfg = Config::default();
                    let s = if let Ok(s) = serde_yml::to_string(&cfg) {
                        s
                    } else {
                        panic!("failed to load config: {e}");
                    };
                    std::fs::write(config_dir().join(format!("{}.yml", config_name)), s).unwrap();
                } else {
                    panic!("failed to load config: {e}");
                }
            } else {
                panic!("failed to load config: {e}");
            }
        }
        _ => (),
    }
    //open cfg
    let editor = if let Some(editor) = env::var_os("EDITOR") {
        editor
    } else {
        panic!("$EDITOR not set")
    };
    let err = Command::new(editor)
        .args([config_dir().join(format!("{}.yml", config_name))])
        .exec();
    panic!("failed to exec editor: {err}");
}

pub fn remove_config(config_name: &str) {
    if let Err(e) = std::fs::remove_file(config_dir().join(format!("{}.yml", config_name))) {
        panic!("failed to remove config: {e}");
    }
}

pub fn list_configs() {
    for entry in std::fs::read_dir(config_dir()).unwrap() {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let file_name = file_name.into_string().unwrap();
        if file_name.ends_with(".yml") {
            println!("{}", file_name.trim_end_matches(".yml"));
        }
    }
}
