use anyhow::Result;
use clap::Args;

use crate::podman::delete_litterbox;

/// Delete an existing Litterbox
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to delete
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        delete_litterbox(&self.name)?;

        Ok(())
    }
}
