use crate::keys::Keys;
use anyhow::Result;
use clap::Args;

/// List all the keys that are being managed
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub fn run(self, keys: Keys) -> Result<()> {
        keys.print_list();

        Ok(())
    }
}
