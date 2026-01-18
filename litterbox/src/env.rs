use crate::errors::LitterboxError;

fn get_env(lbx_name: &'static str) -> Result<String, LitterboxError> {
    std::env::var_os(lbx_name)
        .ok_or(LitterboxError::EnvVarUndefined(lbx_name))?
        .into_string()
        .map_err(|value| LitterboxError::EnvVarInvalid(lbx_name, value))
}

pub fn home_dir() -> Result<String, LitterboxError> {
    get_env("HOME")
}

pub fn wayland_display() -> Result<String, LitterboxError> {
    get_env("WAYLAND_DISPLAY")
}

pub fn xdg_runtime_dir() -> Result<String, LitterboxError> {
    get_env("XDG_RUNTIME_DIR")
}
