use anyhow::Result;
use clap::Args;

use crate::podman::{build_image, build_litterbox};

/// Build a new Litterbox
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to build
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        build_image(&self.name)?;
        build_litterbox(&self.name)?;

        Ok(())
    }
}
