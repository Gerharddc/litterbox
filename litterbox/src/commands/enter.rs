use std::{ffi::OsString, path::PathBuf};

use anyhow::Result;
use clap::Args;

/// Enter an existing Litterbox
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to enter
    name: String,

    /// Make STDIN available to the contained process. Defaults to "true" if
    /// COMMAND is not supplied
    #[arg(long, short, default_value_t = false)]
    interactive: bool,

    /// Allocate a pseudo-TTY. Defaults to "true" if COMMAND is not supplied
    #[arg(long, short, default_value_t = false)]
    tty: bool,

    /// Working directory inside the container
    #[arg(long, short)]
    workdir: Option<PathBuf>,

    /// Run as root inside the container
    #[arg(long, default_value_t = false)]
    root: bool,

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
