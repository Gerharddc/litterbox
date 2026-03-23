use crate::keys::Keys;
use anyhow::Result;
use clap::Args;

/// Detach an attached Litterbox from a key
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the key
    key_name: String,
}

impl Command {
    pub fn run(self, mut keys: Keys) -> Result<()> {
        keys.detach(&self.key_name)?;

        Ok(())
    }
}
