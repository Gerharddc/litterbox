use std::path::Path;

use crate::{LitterboxError, get_env};

pub fn path_relative_to_home(relative_path: &str) -> Result<String, LitterboxError> {
    let home_dir = get_env("HOME")?;
    let home_path = Path::new(&home_dir);
    let full_path = home_path.join(relative_path);

    // TODO: maybe don't do the lossy conversion?
    Ok(full_path.to_string_lossy().to_string())
}

pub fn dockerfile_path(lbx_name: &str) -> Result<String, LitterboxError> {
    path_relative_to_home(&format!("Litterbox/{lbx_name}.Dockerfile"))
}
