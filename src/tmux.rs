use std::collections::BTreeMap;
use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::config::{CmdPanes, Layout, Pane, Panes, Window};
use crate::util::{expand_tilde, home_dir};

pub struct Tmux {
    pub session: String,
    pub root_dir: Option<String>,
}

impl Tmux {
    fn output(args: &[&str]) -> String {
        let out = Command::new("tmux")
            .args(args)
            .output()
            .expect("failed to exec tmux");
        if !out.status.success() {
            panic!("tmux failed: {}", String::from_utf8_lossy(&out.stderr));
        }
        String::from_utf8_lossy(&out.stdout).to_string()
    }

    pub fn foreground_session_name() -> String {
        let session = Self::get_current_session_name().expect("mux save must run inside tmux");
        let attached = Self::output(&["display-message", "-p", "-F", "#{session_attached}"]);
        assert!(
            attached.trim() != "0",
            "mux save requires an attached tmux session"
        );
        session
    }

    pub fn config_name() -> Option<String> {
        let name = Self::output(&["display-message", "-p", "-F", "#{@mux_config}"]);
        let name = name.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    }

    pub fn current_windows(session: &str, previous: &[Window]) -> Vec<Window> {
        let current_pane = std::env::var("TMUX_PANE").expect("mux save must run in a tmux pane");
        let mut windows = Vec::new();
        for line in Self::output(&[
            "list-windows",
            "-t",
            session,
            "-F",
            "#{window_id}\t#{window_name}",
        ])
        .lines()
        {
            let (window_id, name) = line.split_once('\t').unwrap();
            let mut panes = Vec::new();
            for line in Self::output(&[
                "list-panes",
                "-t",
                window_id,
                "-F",
                "#{pane_id}\t#{pane_title}",
            ])
            .lines()
            {
                let (pane_id, title) = line.split_once('\t').unwrap();
                let commands = if pane_id == current_pane {
                    Vec::new()
                } else {
                    let saved = Self::output(&[
                        "show-option",
                        "-pqv",
                        "-t",
                        pane_id,
                        "@mux_running_command",
                    ]);
                    let saved = saved.strip_suffix('\n').unwrap_or(&saved);
                    if saved.is_empty() {
                        Vec::new()
                    } else {
                        vec![saved.to_string()]
                    }
                };
                panes.push(Pane::PaneWithCommands(BTreeMap::from([(
                    title.to_string(),
                    commands,
                )])));
            }
            let (layout, root) = previous
                .iter()
                .find_map(|window| match window {
                    Window::WindowWithPanes(map) => match map.get(name) {
                        Some(CmdPanes::Panes(panes)) => {
                            Some((panes.layout.clone(), panes.root.clone()))
                        }
                        _ => None,
                    },
                    _ => None,
                })
                .unwrap_or((None, None));
            windows.push(Window::WindowWithPanes(BTreeMap::from([(
                name.to_string(),
                CmdPanes::Panes(Panes {
                    panes,
                    layout,
                    root,
                }),
            )])));
        }
        windows
    }

    pub fn new(session: &str, root_dir: &str) -> Tmux {
        let t = Tmux {
            session: String::from(session),
            root_dir: Option::from(String::from(root_dir)),
        };
        t
    }

    pub fn start_in_background(&self) {
        if self.is_session_exist() {
            return;
        }

        let root_dir = if let Some(root_dir) = &self.root_dir {
            root_dir
        } else {
            "."
        };
        let root_dir = expand_tilde(&root_dir);
        match Command::new("tmux")
            .args(["new", "-d", "-s", &self.session, "-c", root_dir.as_str()])
            .output()
        {
            Ok(out) => {
                println!("{}", String::from_utf8_lossy(&out.stdout));
            }
            Err(err) => panic!("failed to exec tmux: {err}"),
        }
    }

    pub fn set_config_name(&self, config_name: &str) {
        Self::output(&[
            "set-option",
            "-t",
            &self.session,
            "@mux_config",
            config_name,
        ]);
    }

    fn get_current_session_name() -> io::Result<String> {
        if !Self::is_in_tmux() {
            return Err(io::Error::new(io::ErrorKind::Other, "Not in tmux"));
        }

        let out = Command::new("tmux")
            .args(["display-message", "-p", "-F", "#{session_name}"])
            .output()?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(io::Error::new(io::ErrorKind::Other, err.trim().to_string()));
        }

        Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    }

    fn get_current_window_idx() -> io::Result<u32> {
        if !Self::is_in_tmux() {
            return Err(io::Error::new(io::ErrorKind::Other, "Not in tmux"));
        }

        let out = Command::new("tmux")
            .args(["display-message", "-p", "-F", "#{window_index}"])
            .output()?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(io::Error::new(io::ErrorKind::Other, err.trim().to_string()));
        }

        Ok(String::from_utf8_lossy(&out.stdout)
            .trim_end()
            .to_string()
            .parse()
            .unwrap())
    }

    fn get_current_pane_idx() -> io::Result<u32> {
        if !Self::is_in_tmux() {
            return Err(io::Error::new(io::ErrorKind::Other, "Not in tmux"));
        }

        let out = Command::new("tmux")
            .args(["display-message", "-p", "-F", "#{pane_index}"])
            .output()?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(io::Error::new(io::ErrorKind::Other, err.trim().to_string()));
        }

        Ok(String::from_utf8_lossy(&out.stdout)
            .trim_end()
            .to_string()
            .parse()
            .unwrap())
    }

    fn is_in_tmux() -> bool {
        std::env::var_os("TMUX")
            .and_then(|v| if v.is_empty() { None } else { Some(v) })
            .is_some()
    }

    pub fn is_session_exist(&self) -> bool {
        let out = Command::new("tmux")
            .args(["has-session", "-t", &self.session])
            .output();
        // println!("out: {out:?}");
        let out = String::from_utf8(out.unwrap().stderr).unwrap();
        // println!("{}", out.contains("can't find session"));
        return !out.contains("can't find session") && !out.contains("no server running");
    }

    pub fn attach_or_switch(&self) {
        let err = if Self::is_in_tmux() {
            Command::new("tmux")
                .args(["switch-client", "-t", &self.session])
                .exec()
        } else {
            Command::new("tmux")
                .args(["attach", "-t", &self.session])
                .exec()
        };
        panic!("failed to exec tmux: {err}");
    }

    pub fn add_window(&self, name: &str, dir: &Option<String>) {
        let root_dir = if let Some(root_dir) = dir {
            String::from(root_dir)
        } else {
            if let Some(root_dir) = &self.root_dir {
                String::from(root_dir)
            } else {
                home_dir()
            }
        };
        let root_dir = expand_tilde(&root_dir);
        // println!("root_dir: {root_dir}");
        let err = Command::new("tmux")
            .args([
                "new-window",
                "-t",
                &self.session,
                "-n",
                name,
                "-c",
                &root_dir,
            ])
            .output()
            .err();
        if let Some(err) = err {
            panic!("failed to exec tmux: {err}");
        }
    }

    pub fn remove_window(&self, window: &str) {
        let err = Command::new("tmux")
            .args(["kill-window", "-t", window])
            .output()
            .err();
        if let Some(err) = err {
            panic!("failed to exec tmux: {err}");
        }
    }

    pub fn set_renumber_windows(&self, flag: bool) {
        let flag_str = if flag { "on" } else { "off" };
        let err = Command::new("tmux")
            .args(["set", "-t", &self.session, "renumber-windows", flag_str])
            .output()
            .err();
        if let Some(err) = err {
            panic!("failed to exec tmux: {err}");
        }
    }

    pub fn split_window(
        &self,
        window: &str,
        name: Option<&str>,
        layout: &Option<&Layout>,
    ) -> String {
        let split_method = match layout {
            Some(layout) => match layout {
                Layout::EvenHorizontal => "-h",
                Layout::EvenVertical => "-v",
            },
            None => "-h",
        };

        let out = Command::new("tmux")
            .args([
                "split-window",
                "-t",
                window,
                split_method,
                "-c",
                "#{pane_current_path}",
                "-P",
                "-F",
                "#{pane_index}",
            ])
            .output();
        if let Ok(out) = out {
            let mut id: String = String::from_utf8_lossy(&out.stdout).to_string();
            id = id.trim_end().to_string();
            let name = if let Some(name) = name {
                name
            } else {
                return id;
            };

            let err = Command::new("tmux")
                .args(["select-pane", "-t", &id, "-T", name])
                .output()
                .err();
            if let Some(err) = err {
                panic!("failed to exec tmux: {err}");
            }
            return id;
        } else {
            let err = out.err().unwrap();
            panic!("failed to exec tmux: {err}");
        }
    }

    pub fn send_cmd(&self, target: &str, cmd: &str) {
        let err = Command::new("tmux")
            .args(["send-keys", "-t", target, "-l", cmd])
            .output()
            .err();
        if let Some(err) = err {
            panic!("failed to exec tmux: {err}");
        }

        let err = Command::new("tmux")
            .args(["send-keys", "-t", target, "Enter"])
            .output()
            .err();
        if let Some(err) = err {
            panic!("failed to exec tmux: {err}");
        }
    }

    pub fn kill_session() {
        let session = match Self::get_current_session_name() {
            Ok(name) => name,
            Err(err) => {
                panic!("failed to exec tmux: {err}");
            }
        };
        let err = Command::new("tmux")
            .args(["kill-session", "-t", &session])
            .exec();
        panic!("failed to exec tmux: {err}");
    }

    pub fn kill_window() {
        let session = match Self::get_current_session_name() {
            Ok(name) => name,
            Err(err) => {
                panic!("failed to exec tmux: {err}");
            }
        };
        let window_idx = match Self::get_current_window_idx() {
            Ok(idx) => idx,
            Err(err) => {
                panic!("failed to exec tmux: {err}");
            }
        };
        let err = Command::new("tmux")
            .args([
                "kill-window",
                "-t",
                format!("{}:{}", &session, window_idx).as_str(),
            ])
            .exec();
        panic!("failed to exec tmux: {err}");
    }

    pub fn kill_pane() {
        let session = match Self::get_current_session_name() {
            Ok(name) => name,
            Err(err) => {
                panic!("failed to exec tmux: {err}");
            }
        };
        let window_idx = match Self::get_current_window_idx() {
            Ok(idx) => idx,
            Err(err) => {
                panic!("failed to exec tmux: {err}");
            }
        };
        let pane_idx = match Self::get_current_pane_idx() {
            Ok(idx) => idx,
            Err(err) => {
                panic!("failed to exec tmux: {err}");
            }
        };
        let err = Command::new("tmux")
            .args([
                "kill-pane",
                "-t",
                format!("{}:{}.{}", &session, window_idx, pane_idx).as_str(),
            ])
            .exec();
        panic!("failed to exec tmux: {err}");
    }
}
