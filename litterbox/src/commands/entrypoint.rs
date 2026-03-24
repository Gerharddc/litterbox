use anyhow::Result;
use clap::Args;
use nix::unistd::{Gid, Uid};

use crate::entrypoint::{CommonEntrypointOptions, run_entrypoint};

/// Container entrypoint (for internal use)
#[derive(Args, Debug)]
pub struct Command {
    /// The UID to drop to if dropping privileges
    #[arg(long, value_parser = |x: &str| x.parse().map(Uid::from_raw))]
    uid: Uid,

    /// The GID to drop to if dropping privileges
    #[arg(long, value_parser = |x: &str| x.parse().map(Gid::from_raw))]
    gid: Gid,

    #[clap(flatten)]
    opts: CommonEntrypointOptions,
}

impl Command {
    pub fn run(self) -> Result<()> {
        // SAFETY: run_entrypoint runs in a single-threaded process.
        unsafe { run_entrypoint(self.uid, self.gid, self.opts) }
    }
}
