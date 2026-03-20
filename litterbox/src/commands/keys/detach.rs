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
    pub fn run(self, keys: Keys) -> Result<()> {
        Ok(())
    }
}
