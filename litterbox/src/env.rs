use anyhow::{Context, Result};

fn get_env(lbx_name: &'static str) -> Result<String> {
    let value = std::env::var(lbx_name)
        .with_context(|| format!("Environment variable {lbx_name} is not defined"))?;
    Ok(value)
}

pub fn home_dir() -> Result<String> {
    get_env("HOME")
}

pub fn wayland_display() -> Result<String> {
    get_env("WAYLAND_DISPLAY")
}

pub fn xdg_runtime_dir() -> Result<String> {
    get_env("XDG_RUNTIME_DIR")
}

pub fn shell() -> Result<String> {
    get_env("SHELL")
}

pub fn litterbox_binary_path() -> String {
    std::env::args()
        .next()
        .expect("Binary path should be defined.")
}
