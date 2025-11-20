use log::{debug, info};
use nix::sys::stat::{SFlag, major, minor, stat};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{errors::LitterboxError, files::lbx_home_path};

fn mknod(
    major_num: u64,
    minor_num: u64,
    dev_type: &str,
    path: &Path,
) -> Result<(), LitterboxError> {
    println!(
        "Root permissions are required to create a device node. Please enter your password if prompted."
    );

    let mut child = Command::new("sudo")
        .args([
            "mknod",
            &path.to_string_lossy(), // TODO: maybe do something else instead?
            dev_type,
            &major_num.to_string(),
            &minor_num.to_string(),
        ])
        .spawn()
        .map_err(LitterboxError::RunPodman)?;

    // FIXME: create dedicated error
    let res = child.wait().map_err(LitterboxError::RunPodman)?;
    debug!("res: {:#?}", res);

    // FIXME: create dedicated error
    if !res.success() {
        panic!("{}", res.to_string());
    }
    Ok(())
}

pub fn attach_device(lbx_name: &str, device_path: &str) -> Result<PathBuf, LitterboxError> {
    let sub_path = device_path
        .strip_prefix("/dev/")
        .ok_or(LitterboxError::InvalidDevicePath(device_path.to_string()))?;
    debug!("sub_path: {:#?}", sub_path);

    let lbx_path = lbx_home_path(lbx_name)?;
    debug!("lbx_path: {:#?}", lbx_path);
    let dest_path = lbx_path.join("dev").join(sub_path);
    debug!("dest_path: {:#?}", dest_path);

    let metadata = stat(device_path).map_err(LitterboxError::Nix)?;
    let rdev = metadata.st_rdev;
    let kind = SFlag::from_bits_truncate(metadata.st_mode);

    let major_num = major(rdev);
    let minor_num = minor(rdev);
    let dev_type = match kind {
        t if t.contains(SFlag::S_IFBLK) => "b",
        t if t.contains(SFlag::S_IFCHR) => "c",
        _ => "unknown",
    };

    debug!("Device Path: {}", device_path);
    info!(
        "Device Type: {}, Major: {}, Minor: {}",
        dev_type, major_num, minor_num
    );

    // Ensure that the path for the destination file exists
    let output_dir = dest_path
        .parent()
        .expect("Destination path should have parent.");
    fs::create_dir_all(output_dir)
        .map_err(|e| LitterboxError::DirUncreatable(e, output_dir.to_path_buf()))?;
    debug!("Output dir ready!");

    mknod(major_num, minor_num, dev_type, &dest_path)?;
    // TODO: maybe we also need to set the owner and permissions
    Ok(dest_path)
}
