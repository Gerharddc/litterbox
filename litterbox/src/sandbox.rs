use anyhow::{Context, Result};
use landlock::{
    ABI, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, path_beneath_rules,
};
use nix::unistd::{Gid, Uid, chown, setgid, setuid};
use std::{
    ffi::OsString,
    os::unix::{fs::symlink, process::CommandExt},
    path::Path,
    process::Command,
};

use crate::{
    env::{self, xdg_runtime_dir},
    files::setup_home,
};

pub fn apply_landlock() -> Result<()> {
    let access_all = AccessFs::from_all(ABI::V6);

    let ruleset = Ruleset::default();
    let ruleset = ruleset.handle_access(access_all)?;

    // We avoid giving full access to the container's entire root directory so that we can
    // deny access to "internal" files that Litterbox places within the root directory.
    let read_dir = std::fs::read_dir("/")?;
    let paths: Vec<_> = read_dir
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.is_dir() { Some(path) } else { None }
        })
        .collect();

    let ruleset = ruleset.create()?;
    let ruleset = ruleset.add_rules(path_beneath_rules(paths, access_all))?;
    let ruleset = ruleset.add_rules(path_beneath_rules(["/"], AccessFs::ReadDir))?;
    let ruleset = ruleset.add_rules(path_beneath_rules(
        ["/litterbox", "/prep-home.sh"],
        AccessFs::Execute | AccessFs::ReadFile,
    ))?;

    match ruleset.restrict_self() {
        Ok(status) => {
            eprintln!(
                "Landlock sandbox applied: {:?}, no_new_privs: {}",
                status.ruleset, status.no_new_privs
            );
        }
        Err(e) => {
            eprintln!("Failed to apply Landlock sandbox: {:?}", e);
        }
    }

    Ok(())
}

pub fn entrypoint(
    root: bool,
    uid: Uid,
    gid: Gid,
    command: Option<OsString>,
    args: Vec<OsString>,
) -> Result<()> {
    let run0_path = Path::new("/usr/bin/run0");
    if !run0_path.exists() {
        symlink("/litterbox", run0_path)?;
    }

    let sudo_path = Path::new("/usr/bin/sudo");
    if !sudo_path.exists() {
        symlink("/litterbox", sudo_path)?;
    }

    let xdg_runtime_dir = xdg_runtime_dir().context("$XDG_RUNTIME_DIR is not set")?;

    chown(&xdg_runtime_dir, Some(uid), Some(gid))
        .context("Failed to set owner of $XDG_RUNTIME_DIR")?;

    if !root {
        setgid(gid)?;
        setuid(uid)?;
        eprintln!("Dropped permissions to non-root user");
    }

    apply_landlock()?;
    setup_home()?;

    let mut cmd = Command::new(&env::shell()?);
    cmd.arg("-l");

    if let Some(mut command) = command {
        // We can't use Command::args for "command" because shells
        // generally expect a single argument for the "-c" option
        for arg in args {
            command.push(" ");
            command.push(arg);
        }

        cmd.arg("-c");
        cmd.arg(command);
    }

    // On success it never returns
    let cause = cmd.exec();

    println!(
        "Failed to execute program '{}': {cause}",
        cmd.get_program().to_string_lossy()
    );

    Ok(())
}
