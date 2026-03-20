use anyhow::Result;
use clap::Args;

/// Attach a device to a Litterbox (the device fille be created in the home directory)
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to attach the device to
    name: String,

    /// The path of the device to be attached
    path: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
