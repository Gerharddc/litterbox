use std::ffi::OsString;

use anyhow::Result;
use clap::Args;
use nix::unistd::{Gid, Uid};

/// Container entrypoint (for internal use)
#[derive(Args, Debug)]
pub struct Command {
    /// Run as root instead of dropping privileges
    #[arg(long, default_value_t = false)]
    root: bool,

    /// The UID to drop to if dropping privileges
    #[arg(long)]
    uid: Uid,

    /// The GID to drop to if dropping privileges
    #[arg(long)]
    gid: Gid,

    /// The command to execute instead of the login shell
    command: Option<OsString>,

    /// Additional arguments passed to the command
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<OsString>,
}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
