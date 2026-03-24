use crate::{entrypoint::CommonEntrypointOptions, env, files::setup_home};
use anyhow::{Context, Result, bail};
use landlock::{
    ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, path_beneath_rules,
};
use log::{debug, error, info, warn};
use nix::{
    sys::{
        prctl::set_child_subreaper,
        signal::{Signal, killpg},
        wait::{WaitPidFlag, WaitStatus, waitpid},
    },
    unistd::{Gid, Pid, Uid, chown, getpgrp, setgid, setuid},
};
use std::{
    os::unix::{fs::symlink, prelude::ExitStatusExt},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

const LOGIN_SHELL_FINISHED_MSG: &str =
    "Login shell has finished, but there are processes running in the background";

pub fn apply_landlock() -> Result<()> {
    // We avoid giving full access to the container's entire root directory so
    // that we can deny access to "internal" files that Litterbox places within
    // the root directory.
    let root_dirs = std::fs::read_dir("/")?.filter_map(|entry| {
        let path = entry.ok()?.path();

        path.is_dir().then_some(path)
    });

    let access_all = AccessFs::from_all(ABI::V6);
    let ruleset = Ruleset::default()
        .handle_access(access_all)?
        .create()?
        .add_rules(path_beneath_rules(root_dirs, access_all))?
        .add_rules(path_beneath_rules(["/"], AccessFs::ReadDir))?
        .add_rules(path_beneath_rules(
            ["/litterbox", "/prep-home.sh"],
            AccessFs::Execute | AccessFs::ReadFile,
        ))?;

    match ruleset.restrict_self() {
        Ok(status) => debug!("Landlock sandbox applied: {status:?}"),
        Err(cause) => error!("Failed to apply Landlock sandbox: {cause:?}"),
    }

    Ok(())
}

pub fn entrypoint(uid: Uid, gid: Gid, opts: CommonEntrypointOptions) -> Result<()> {
    let run0_path = Path::new("/usr/bin/run0");
    if !run0_path.exists() {
        symlink("/litterbox", run0_path)?;
    }

    let sudo_path = Path::new("/usr/bin/sudo");
    if !sudo_path.exists() {
        symlink("/litterbox", sudo_path)?;
    }

    let xdg_runtime_dir = env::xdg_runtime_dir().context("$XDG_RUNTIME_DIR is not set")?;

    chown(&xdg_runtime_dir, Some(uid), Some(gid))
        .context("Failed to set owner of $XDG_RUNTIME_DIR")?;

    if !opts.root {
        setgid(gid)?;
        setuid(uid)?;
        debug!("Dropped from root to {uid}:{gid}");
    } else {
        debug!("Will keep root privileges!");
    }

    apply_landlock()?;
    setup_home()?;

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

                        // TODO: Daemonize
                        //
                        // NOTE: What about the actual init process `litterbox
                        // wait` command? Do I merge them together?
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
