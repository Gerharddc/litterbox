use anyhow::Result;
use clap::Args;

/// Run daemon (for internal use)
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
