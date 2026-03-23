use anyhow::Result;
use clap::Args;

use crate::devices::attach_device;

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
        let dest_path = attach_device(&self.name, &self.path)?;
        println!("Device attached at {:#?}!", dest_path);

        Ok(())
    }
}
