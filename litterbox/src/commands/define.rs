use anyhow::Result;
use clap::Args;

/// Define a new Litterbox using a template Dockerfile
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to define
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
