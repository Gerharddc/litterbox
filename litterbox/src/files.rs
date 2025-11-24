use std::fs;
use std::path::{Path, PathBuf};

use crate::{LitterboxError, get_env};

fn path_relative_to_lbx_root(relative_path: &str) -> Result<PathBuf, LitterboxError> {
    let home_dir = get_env("HOME")?;
    let home_path = Path::new(&home_dir);
    let full_path = home_path.join("Litterbox").join(relative_path);
    Ok(full_path)
}

pub fn dockerfile_path(lbx_name: &str) -> Result<PathBuf, LitterboxError> {
    path_relative_to_lbx_root(&format!("definitions/{lbx_name}.Dockerfile"))
}

pub fn keyfile_path() -> Result<PathBuf, LitterboxError> {
    path_relative_to_lbx_root("keys.ron")
}

pub fn lbx_home_path(lbx_name: &str) -> Result<PathBuf, LitterboxError> {
    path_relative_to_lbx_root(&format!("homes/{lbx_name}"))
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

pub struct SshSockFile {
    path: PathBuf,
}

impl SshSockFile {
    pub fn new(lbx_name: &str, create_empty_placeholder: bool) -> Result<Self, LitterboxError> {
        let path = path_relative_to_lbx_root(&format!(".ssh/{lbx_name}.sock"))?;
        let path_ref = &path;

        if fs::exists(path_ref).map_err(|e| LitterboxError::ExistsFailed(e, path.clone()))? {
            log::warn!("Deleting old SSH socket: {:#?}", path_ref);
            fs::remove_file(path_ref).map_err(|e| LitterboxError::RemoveFailed(e, path.clone()))?;
        } else {
            let ssh_dir = path_ref.parent().expect("SSH path should have parent.");
            fs::create_dir_all(ssh_dir)
                .map_err(|e| LitterboxError::DirUncreatable(e, ssh_dir.to_path_buf()))?;

            if create_empty_placeholder {
                fs::File::create(path_ref)
                    .map_err(|e| LitterboxError::CreateFailed(e, ssh_dir.to_path_buf()))?;
            }
        }

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SshSockFile {
    fn drop(&mut self) {
        if let Ok(true) = fs::exists(self.path.clone()) {
            if let Err(e) = fs::remove_file(self.path.clone()) {
                log::error!("Failed to remove {:#?}, error: {:#?}", self.path, e);
            }
        } else {
            log::error!("No SSH socket file to clean up: {:#?}", self.path());
        }
    }
}
