use std::os::unix::process::CommandExt;
use std::process::Command;
use std::{env, io};

use crate::config::Layout;
use crate::util::{expand_tilde, home_dir};

pub struct Tmux {
    pub session: String,
    pub root_dir: Option<String>,
}

impl Tmux {
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
        match Command::new("tmux")
            .args(["new", "-d", "-t", &self.session, "-c", root_dir])
            .output()
        {
            Ok(out) => {
                println!("{}", String::from_utf8_lossy(&out.stdout));
            }
            Err(err) => panic!("failed to exec tmux: {err}"),
        }
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
        println!("out: {out:?}");
        let out = String::from_utf8(out.unwrap().stderr).unwrap();
        println!("{}", out.contains("can't find session"));
        return !out.contains("can't find session") && !out.contains("no server running");
    }

    pub fn attach_or_switch(&self) {
        let err = if Self::is_in_tmux() {
            Command::new("tmux")
                .args(["switch-session", "-t", &self.session])
                .exec()
        } else {
            Command::new("tmux")
                .args(["attach", "-t", &self.session])
                .exec()
        };
        eprintln!("failed to exec tmux: {err}");
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
        eprintln!("failed to exec tmux: {err}");
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
