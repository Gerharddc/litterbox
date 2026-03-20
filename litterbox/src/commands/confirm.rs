use anyhow::Result;
use clap::Args;

/// Ask the user to confirm a request (for internal use)
#[derive(Args, Debug)]
pub struct Command {
    // The request that the user needs to confirm
    #[arg(long)]
    request: String,

    // The name of the litterbox sending the request
    #[arg(long)]
    lbx_name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        Ok(())
    }
}
