use anyhow::Result;
use clap::Args;
use nix::unistd::{Gid, Uid};

use crate::{entrypoint::CommonEntrypointOptions, sandbox::entrypoint};

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
        entrypoint(self.uid, self.gid, self.opts)
    }
}
