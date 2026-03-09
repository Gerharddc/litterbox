use anyhow::{Context, Result};
use std::fs::{self, File};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::env;

fn path_relative_to_lbx_root(relative_path: &str) -> Result<PathBuf> {
    let home_dir = env::home_dir()?;
    let home_path = Path::new(&home_dir);
    let full_path = home_path.join("Litterbox").join(relative_path);
    Ok(full_path)
}

pub fn dockerfile_path(lbx_name: &str) -> Result<PathBuf> {
    path_relative_to_lbx_root(&format!("definitions/{lbx_name}.Dockerfile"))
}

pub fn keyfile_path() -> Result<PathBuf> {
    path_relative_to_lbx_root("keys.ron")
}

pub fn lbx_home_path(lbx_name: &str) -> Result<PathBuf> {
    path_relative_to_lbx_root(&format!("homes/{lbx_name}"))
}

pub fn settings_path(lbx_name: &str) -> Result<PathBuf> {
    path_relative_to_lbx_root(&format!("definitions/{lbx_name}.ron"))
}

pub fn daemon_lock_path(lbx_name: &str) -> Result<PathBuf> {
    path_relative_to_lbx_root(&format!(".daemon-{lbx_name}.lock"))
}

pub fn session_lock_path(lbx_name: &str) -> Result<PathBuf> {
    path_relative_to_lbx_root(&format!(".session-{lbx_name}.lock"))
}

pub fn daemon_log_file(lbx_name: &str) -> Result<File> {
    let path = path_relative_to_lbx_root(&format!("logs/daemon-{lbx_name}.log"))?;
    let output_dir = path.parent().expect("Path should have parent.");
    fs::create_dir_all(output_dir)?;
    File::create(&path).context("Could not create daemon log file")
}

pub fn append_pid_to_session_lockfile(path: &Path, pid: u32) -> Result<()> {
    let mut pids = read_pids_from_session_lockfile(path)?;

    if !pids.contains(&pid) {
        pids.push(pid);
        write_pids_to_session_lockfile(path, &pids)?;
    }

    Ok(())
}

pub fn remove_pid_from_session_lockfile(path: &Path, pid: u32) -> Result<()> {
    let mut pids = read_pids_from_session_lockfile(path)?;

    pids.retain(|&p| p != pid);
    write_pids_to_session_lockfile(path, &pids)?;

    Ok(())
}

pub fn write_pids_to_session_lockfile(path: &Path, pids: &[u32]) -> Result<()> {
    let content = pids
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    write_file(path, &content)
}

pub fn read_pids_from_session_lockfile(path: &Path) -> Result<Vec<u32>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = read_file(path)?;
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().parse::<u32>().map_err(anyhow::Error::from))
        .collect()
}

pub fn cleanup_dead_pids_from_session_lockfile(path: &Path) -> Result<()> {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    if !path.exists() {
        return Ok(());
    }

    let pids = read_pids_from_session_lockfile(path)?;
    let alive_pids: Vec<u32> = pids
        .iter()
        .filter(|&pid| {
            let pid = Pid::from_raw(*pid as i32);
            kill(pid, None).is_ok()
        })
        .copied()
        .collect();

    if pids == alive_pids {
        return Ok(());
    }

    write_pids_to_session_lockfile(path, &alive_pids)?;

    Ok(())
}

pub fn pipewire_socket_path() -> Result<PathBuf> {
    let xdg_runtime_dir = env::xdg_runtime_dir()?;
    let path = format!("{xdg_runtime_dir}/pipewire-0");
    Ok(Path::new(&path).to_path_buf())
}

pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    let output_dir = path.parent().expect("Path should have parent.");
    fs::create_dir_all(output_dir)?;
    fs::write(path, contents)?;
    Ok(())
}

pub fn read_file(path: &Path) -> Result<String> {
    Ok(fs::read_to_string(path)?)
}

pub fn wait_for_sessions_to_finish() -> Result<()> {
    use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};

    let session_lock_path = Path::new("/session.lock");
    let is_empty = || match std::fs::read_to_string(session_lock_path) {
        Ok(content) => content.trim().is_empty(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => {
            log::error!("Failed to read session lock file: {}", e);
            false
        }
    };

    if is_empty() {
        println!("Session already empty, Litterbox finished.");
        return Ok(());
    }

    let inotify = Inotify::init(InitFlags::empty())?;
    inotify.add_watch(session_lock_path, AddWatchFlags::IN_MODIFY)?;

    println!("Litterbox started, waiting for session to become empty.");
    loop {
        let _ = inotify.read_events()?;
        if is_empty() {
            break;
        }
    }
    println!("Session empty, Litterbox finished.");

    Ok(())
}

pub struct SshSockFile {
    path: PathBuf,
}

impl SshSockFile {
    pub fn new(lbx_name: &str, create_empty_placeholder: bool) -> Result<Self> {
        let path = path_relative_to_lbx_root(&format!(".ssh/{lbx_name}.sock"))?;
        let path_ref = &path;

        if fs::exists(path_ref)? {
            log::warn!("Deleting old SSH socket: {:#?}", path_ref);
            fs::remove_file(path_ref)?;
        } else {
            let ssh_dir = path_ref.parent().expect("SSH path should have parent.");
            fs::create_dir_all(ssh_dir)?;

            if create_empty_placeholder {
                fs::File::create(path_ref)?;
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

pub fn setup_home() -> Result<()> {
    let home = env::home_dir()?;
    let marker = format!("{}/.home-built", home);

    if Path::new(&marker).exists() {
        println!("Home already built; skipping.");
    } else {
        println!("Building home for the first time...");

        if Path::new("/prep-home.sh").exists() {
            Command::new("/prep-home.sh")
                .status()
                .context("Failed to run /prep-home.sh")?;
        }

        File::create(&marker)?;
        println!("Done.");
    }

    let shell = env::shell()?;
    let _ = Command::new(&shell).arg("-l").exec();
    Ok(())
}
