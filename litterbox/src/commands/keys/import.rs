use crate::keys::Keys;
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

/// Import a key to Litterbox
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the new key
    name: String,
    /// The file path to the key
    path: PathBuf,
}

impl Command {
    pub fn run(self, mut keys: Keys) -> Result<()> {
        keys.import_key(&self.name, self.path)?;

        Ok(())
    }
}
