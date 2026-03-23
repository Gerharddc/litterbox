use anyhow::Result;
use clap::Args;

use crate::files::wait_for_sessions_to_finish;

/// Wait for the Litterbox to finish (for internal use)
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub fn run(self) -> Result<()> {
        wait_for_sessions_to_finish()?;

        Ok(())
    }
}
