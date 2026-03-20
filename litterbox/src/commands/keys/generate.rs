use crate::keys::Keys;
use anyhow::Result;
use clap::Args;

/// Generate a new random key
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the key
    name: String,
}

impl Command {
    pub fn run(self, keys: Keys) -> Result<()> {
        Ok(())
    }
}
