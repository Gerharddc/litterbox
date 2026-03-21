use anyhow::{Context, Result};
use std::path::PathBuf;

fn get_env(lbx_name: &'static str) -> Result<String> {
    let value = std::env::var(lbx_name)
        .with_context(|| format!("Environment variable {lbx_name} is not defined"))?;
    Ok(value)
}

pub fn home_dir() -> Result<PathBuf> {
    get_env("HOME").map(PathBuf::from)
}

pub fn wayland_display() -> Result<String> {
    get_env("WAYLAND_DISPLAY")
}

pub fn xdg_runtime_dir() -> Result<PathBuf> {
    get_env("XDG_RUNTIME_DIR").map(PathBuf::from)
}

pub fn shell() -> Result<String> {
    get_env("SHELL")
}

pub fn litterbox_binary_path() -> PathBuf {
    std::env::current_exe().expect("Binary path should be defined.")
}
