use crate::keys::Keys;
use anyhow::Result;
use clap::Args;

/// List all the keys are being managed
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub fn run(self, keys: Keys) -> Result<()> {
        Ok(())
    }
}
