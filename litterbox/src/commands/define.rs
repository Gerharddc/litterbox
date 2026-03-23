use anyhow::Result;
use clap::Args;

use crate::podman::define_litterbox;

/// Define a new Litterbox using a template Dockerfile
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to define
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        define_litterbox(&self.name)?;

        Ok(())
    }
}
