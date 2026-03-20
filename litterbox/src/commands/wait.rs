use anyhow::Result;
use clap::Args;

/// Wait for the Litterbox to finish (for internal use)
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
