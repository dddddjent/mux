use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

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

        if t.is_session_exist() {
            return t;
        }

        match Command::new("tmux")
            .args(["-d", "-t", &session, "-c", &root_dir])
            .output()
        {
            Ok(out) => {
                println!("{}", String::from_utf8_lossy(&out.stdout));
            }
            Err(err) => panic!("failed to exec tmux: {err}"),
        }
        return t;
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

    fn is_session_exist(&self) -> bool {
        let out = Command::new("tmux")
            .args(["has-session", "-t", &self.session])
            .output();
        println!("{}", String::from_utf8_lossy(&out.as_ref().unwrap().stdout));
        return out.unwrap().status.success();
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
