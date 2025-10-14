use std::fs;
use std::path::{Path, PathBuf};

use crate::{LitterboxError, get_env};

/// Returns a path relative to the Litterbox home directory (i.e. ~/Litterbox)
pub fn path_relative_to_home(relative_path: &str) -> Result<PathBuf, LitterboxError> {
    let home_dir = get_env("HOME")?;
    let home_path = Path::new(&home_dir);
    let full_path = home_path.join("Litterbox").join(relative_path);

    // TODO: maybe don't do the lossy conversion?
    let full_path = full_path.to_string_lossy().to_string();
    let full_path = PathBuf::from(full_path);
    Ok(full_path)
}

pub fn dockerfile_path(lbx_name: &str) -> Result<PathBuf, LitterboxError> {
    path_relative_to_home(&format!("{lbx_name}.Dockerfile"))
}

pub fn keyfile_path() -> Result<PathBuf, LitterboxError> {
    path_relative_to_home("keys.ron")
}

pub fn write_file(path: &Path, contents: &str) -> Result<(), LitterboxError> {
    let output_dir = path.parent().expect("Path should have parent.");

    fs::create_dir_all(output_dir)
        .map_err(|e| LitterboxError::DirUncreatable(e, output_dir.to_path_buf()))?;

    fs::write(path, contents).map_err(|e| LitterboxError::WriteFailed(e, path.to_path_buf()))?;
    Ok(())
}

pub fn read_file(path: &Path) -> Result<String, LitterboxError> {
    fs::read_to_string(path).map_err(|e| LitterboxError::ReadFailed(e, path.to_path_buf()))
}
