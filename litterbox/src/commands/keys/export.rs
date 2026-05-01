use crate::keys::Keys;
use anyhow::Result;
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct Command {
    key_name: String,
    path: PathBuf,
}

impl Command {
    pub fn run(self, keys: Keys) -> Result<()> {
        keys.export(&self.key_name, &self.path)?;
        Ok(())
    }
}
