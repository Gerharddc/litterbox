use anyhow::Result;
use clap::Args;

/// List all the Litterboxes that have been created
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
