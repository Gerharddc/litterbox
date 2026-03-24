use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::entrypoint::{CommonEntrypointOptions, Interactive, Tty, enter_litterbox};

/// Enter an existing Litterbox
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to enter
    name: String,

    /// Make STDIN available to the contained process. Defaults to "true" if
    /// COMMAND is not supplied
    #[arg(long, short, default_value_t = Interactive(false))]
    interactive: Interactive,

    /// Allocate a pseudo-TTY. Defaults to "true" if COMMAND is not supplied
    #[arg(long, short, default_value_t = Tty(false))]
    tty: Tty,

    /// Working directory inside the container
    #[arg(long, short)]
    workdir: Option<PathBuf>,

    #[clap(flatten)]
    opts: CommonEntrypointOptions,
}

impl Command {
    pub fn run(self) -> Result<()> {
        enter_litterbox(
            &self.name,
            self.interactive,
            self.tty,
            self.workdir,
            self.opts,
        )
    }
}
