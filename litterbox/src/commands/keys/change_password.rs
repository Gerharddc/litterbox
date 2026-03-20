use crate::keys::Keys;
use anyhow::Result;
use clap::Args;

/// Change the password used to encrypt passwords for storage
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub fn run(self, keys: Keys) -> Result<()> {
        Ok(())
    }
}
