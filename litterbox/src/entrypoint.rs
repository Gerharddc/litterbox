use std::{
    ffi::OsString,
    fmt::Display,
    path::PathBuf,
    process::Command,
    str::{FromStr, ParseBoolError},
};

use anyhow::{Context as _, Result, anyhow};
use clap::Args;
use log::{debug, info, warn};
use nix::unistd::{Pid, getgid, getuid};

use crate::{
    daemon, files,
    podman::{
        get_container, is_container_running, start_daemon, wait_for_podman, wait_for_podman_async,
    },
};

#[derive(Clone, Debug, Copy)]
pub struct Tty(pub bool);

impl Display for Tty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Tty {
    type Err = ParseBoolError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

#[derive(Clone, Debug, Copy)]
pub struct Interactive(pub bool);

impl Display for Interactive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for Interactive {
    type Err = ParseBoolError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

// If you add a new field, make sure to pass it inside the container in
// `container_exec_entrypoint`.
#[derive(Args, Debug)]
pub struct CommonEntrypointOptions {
    /// Run as root instead of dropping privileges.
    #[arg(long, default_value_t = false)]
    pub root: bool,

    /// When set to `true`, it will wait for background processes to finish
    /// in the foreground. When set to `false`, it will send SIGKILL to all
    /// background processes. If it's not specified, litterbox will wait for
    /// background processes in the background.
    #[arg(long)]
    pub wait: Option<bool>,

    /// The command to execute with the login shell.
    pub command: Option<OsString>,

    /// Additional arguments to pass to COMMAND.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<OsString>,
}

pub fn enter_litterbox(
    lbx_name: &str,
    interactive: Interactive,
    tty: Tty,
    workdir: Option<PathBuf>,
    opts: CommonEntrypointOptions,
) -> Result<()> {
    let container =
        get_container(lbx_name)?.ok_or_else(|| anyhow!("No container found for '{lbx_name}'"))?;
    let container_id = container.id;

    if !daemon::is_running(lbx_name)? {
        if is_container_running(lbx_name)? {
            warn!("Daemon was not running but container was. Restarting daemon...");
        }

        start_daemon(lbx_name)?;
    }

    let my_pid = Pid::this();
    let session_lock = files::session_lock_path(lbx_name)?;
    files::append_pid_to_session_lockfile(&session_lock, my_pid)?;

    if !is_container_running(lbx_name)? {
        info!("Container is not running yet; starting now...");

        let start_child = Command::new("podman")
            .args(["start", &container_id])
            .spawn()
            .context("Failed to run podman command")?;

        wait_for_podman(start_child)?;
    } else {
        debug!("Container is already running; just attaching...")
    }

    tokio::runtime::Runtime::new()
        .expect("Tokio runtime should start")
        .block_on(container_exec_entrypoint(
            container_id,
            interactive,
            tty,
            workdir,
            opts,
        ))?;

    files::remove_pid_from_session_lockfile(&session_lock, my_pid)?;
    debug!("Litterbox finished.");
    Ok(())
}

async fn container_exec_entrypoint(
    container_id: String,
    interactive: Interactive,
    tty: Tty,
    workdir: Option<PathBuf>,
    opts: CommonEntrypointOptions,
) -> Result<()> {
    use tokio::process::Command;

    let mut exec_child = Command::new("podman");

    exec_child.arg("exec");

    // Assume -t if we are launching the login shell
    if tty.0 || opts.command.is_none() {
        exec_child.arg("--tty");
    }

    // Assume -i if we are launching the login shell
    if interactive.0 || opts.command.is_none() {
        exec_child.arg("--interactive");
    }

    if let Some(workdir) = workdir {
        exec_child.arg("--workdir");
        exec_child.arg(workdir.into_os_string());
    }

    // We always start as root but drop permissions later if needed
    exec_child.arg("--user");
    exec_child.arg("root");

    exec_child.args([
        &container_id,
        "/litterbox",
        "entrypoint",
        "--uid",
        &getuid().to_string(),
        "--gid",
        &getgid().to_string(),
    ]);

    // The entrypoint is responsible for dropping root if needed
    if opts.root {
        exec_child.arg("--root");
    }

    if let Some(wait) = opts.wait {
        exec_child.args(["--wait", &wait.to_string()]);
    }

    if let Some(command) = opts.command {
        exec_child.arg("--");
        exec_child.arg(command);
        exec_child.args(opts.args);
    }

    let mut exec_child = exec_child.spawn().context("Failed to run podman command")?;
    debug!("Entering Litterbox...");

    tokio::select! {
        _ = wait_for_podman_async(&mut exec_child) => {}
        _ = tokio::signal::ctrl_c() => {
            let _ = exec_child.kill().await;
        }
    }

    debug!("Exited Litterbox");

    Ok(())
}
