use anyhow::Result;
use clap::Args;
use std::io::{Read, stdin};

use crate::daemon;

/// Run daemon (for internal use)
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        let mut password = String::new();
        stdin().read_to_string(&mut password)?;
        let password = password.trim();

        // We wait to create the runtime here since only this one command depends on it.
        tokio::runtime::Runtime::new()
            .expect("Tokio runtime should start")
            .block_on(daemon::run(&self.name, password))?;

        Ok(())
    }
}
