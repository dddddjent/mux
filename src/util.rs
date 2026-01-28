use std::env;
use std::path::PathBuf;

pub fn xdg_config_home() -> PathBuf {
    if let Some(p) = env::var_os("XDG_CONFIG_HOME") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    let home = env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config")
}

pub fn home_dir() -> String {
    env::var_os("HOME")
        .unwrap_or_default()
        .into_string()
        .unwrap_or_default()
}

pub fn expand_tilde(p: &str) -> String {
    if p.starts_with("~") {
        home_dir() + &p[1..]
    } else {
        p.to_string()
    }
}
