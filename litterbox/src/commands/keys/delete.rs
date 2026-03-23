use crate::keys::Keys;
use anyhow::Result;
use clap::Args;

/// Delete an existing key
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the key
    name: String,
}

impl Command {
    pub fn run(self, mut keys: Keys) -> Result<()> {
        keys.delete(&self.name)?;

        Ok(())
    }
}
