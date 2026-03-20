use anyhow::Result;
use clap::Args;

/// Delete an existing Litterbox
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to delete
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
