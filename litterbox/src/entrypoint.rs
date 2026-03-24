use crate::{
    daemon, env, files,
    podman::{
        get_container, is_container_running, start_daemon, wait_for_podman, wait_for_podman_async,
    },
    sandbox,
    utils::{SU_BINARIES, trace_arguments},
};
use anyhow::{Context as _, Result, anyhow, bail};
use clap::Args;
use log::{debug, info, warn};
use nix::{
    sys::{
        prctl::set_child_subreaper,
        signal::{Signal, killpg},
        wait::{WaitPidFlag, WaitStatus, waitpid},
    },
    unistd::{Gid, Pid, Uid, chown, getgid, getpgrp, getuid, setgid, setuid},
};
use std::{
    ffi::OsString,
    fmt::Display,
    os::unix::{fs::symlink, prelude::ExitStatusExt},
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
    str::{FromStr, ParseBoolError},
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

        let mut cmd = Command::new("podman");
        cmd.stdout(Stdio::null());
        cmd.args(["start", &container_id]);
        trace_arguments(&cmd);

        let start_child = cmd.spawn().context("Failed to run podman command")?;
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

/// Runs the entrypoint of a container.
///
/// # Safety
///
/// - This function may fork the current process for daemonization. Ensure you
///   call this function in a single-threaded process. For more information see
///   [`pre_exec`]'s note on safety and [nix-rust/nix#2663].
///
/// [nix-rust/nix#2663]: https://github.com/nix-rust/nix/issues/2663
/// [`pre_exec`]: std::os::unix::process::CommandExt::pre_exec
pub unsafe fn run_entrypoint(uid: Uid, gid: Gid, opts: CommonEntrypointOptions) -> Result<()> {
    let xdg_runtime_dir = env::xdg_runtime_dir().context("$XDG_RUNTIME_DIR is not set")?;

    chown(&xdg_runtime_dir, Some(uid), Some(gid))
        .context("Failed to set owner of $XDG_RUNTIME_DIR")?;

    if !opts.root {
        for su_bin in SU_BINARIES {
            let _ = symlink("/litterbox", format!("/usr/bin/{su_bin}"));
        }

        setgid(gid)?;
        setuid(uid)?;
        debug!("Dropped from root to {uid}:{gid}");
    } else {
        debug!("Will keep root privileges!");
    }

    sandbox::apply_landlock()?;
    files::setup_home()?;

    let mut cmd = Command::new(&env::shell()?);
    cmd.stderr(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stdin(Stdio::inherit());
    // $RUST_LOG can be passed from `litterbox build` for development and
    // debugging purposes. We don't want child processes to inherit it.
    cmd.env_remove("RUST_LOG");

    // Have the shell assume it's a login shell.
    cmd.arg("-l");

    if let Some(mut exec_args) = opts.command {
        // We can't use Command::args for "command" because shells generally
        // expect a single argument for the "-c" option.
        for arg in opts.args {
            exec_args.push(" ");
            exec_args.push(arg);
        }

        // Have the shell execute just `exec_args` and then exit.
        cmd.arg("-c");
        cmd.arg(exec_args);
    }

    let shell_child = cmd.spawn().context("Failed to launch shell")?;
    let shell_pid = Pid::from_raw(shell_child.id() as i32);
    let mut waitpid_flags = WaitPidFlag::empty();

    set_child_subreaper(true).context("failed to make process child subreaper")?;

    loop {
        match waitpid(None, Some(waitpid_flags)) {
            Ok(WaitStatus::Exited(pid, status)) => {
                let status = ExitStatus::from_raw(status);

                if pid == shell_pid {
                    debug!(
                        "Login shell {:?} (PID: {pid}) exited: {status}",
                        cmd.get_program()
                    );

                    if !status.success() {
                        bail!("Failed to execute {:?}", cmd.get_program());
                    }

                    // Activate the `WaitStatus::StillAlive` arm.
                    waitpid_flags |= WaitPidFlag::WNOHANG;
                } else {
                    debug!("Child process {pid} exited: {status}");
                }
            }

            Ok(WaitStatus::Signaled(pid, signal, _)) => {
                if pid == shell_pid {
                    warn!(
                        "Login shell {:?} (PID: {pid}) was killed with signal {signal}",
                        cmd.get_program()
                    );

                    // Activate the `WaitStatus::StillAlive` arm.
                    waitpid_flags |= WaitPidFlag::WNOHANG;
                } else {
                    debug!("Child process {pid} was killed with signal {signal}");
                }
            }

            Ok(WaitStatus::StillAlive) => {
                const LOGIN_SHELL_FINISHED_MSG: &str =
                    "Login shell has finished, but there are processes running in the background";

                // Disable this arm.
                waitpid_flags -= WaitPidFlag::WNOHANG;

                match opts.wait {
                    Some(true) => {
                        info!("{LOGIN_SHELL_FINISHED_MSG}. Press CTRL+C to stop them.");
                    }

                    Some(false) => {
                        info!("{LOGIN_SHELL_FINISHED_MSG}. Exiting anyway...");
                        // Kill all descendants of this process forcefully.
                        //
                        // FIXME: Should they be forcefully killed? What about a graceful exit?
                        let _ = killpg(getpgrp(), Some(Signal::SIGKILL));

                        break;
                    }

                    None => {
                        info!("{LOGIN_SHELL_FINISHED_MSG}. Continuing in the background...");

                        // SAFETY: The caller must ensure this function is called from a single-threaded process.
                        #[expect(
                            unused_unsafe,
                            reason = "https://github.com/nix-rust/nix/issues/2663 seeks to mark daemon as unsafe"
                        )]
                        unsafe {
                            nix::unistd::daemon(true, true).context("Failed to daemonize self")?;
                        }
                    }
                }
            }

            Ok(
                status @ (WaitStatus::PtraceEvent(..)
                | WaitStatus::PtraceSyscall(..)
                | WaitStatus::Continued(..)
                | WaitStatus::Stopped(..)),
            ) => {
                warn!("Child signaled with unhandled status: {status:?}");
            }

            Err(nix::errno::Errno::ECHILD) => {
                debug!("Received ECHILD: No remaining unwaited-for child processes");
                break;
            }

            Err(cause) => bail!(cause),
        }
    }

    Ok(())
}
