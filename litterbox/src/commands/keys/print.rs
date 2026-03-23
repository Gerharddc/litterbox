use crate::keys::Keys;
use anyhow::Result;
use clap::Args;

/// Print the key in OpenSSH public key format
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the key
    key_name: String,

    /// Print the private key instead of the public key
    #[clap(long)]
    private: bool,
}

impl Command {
    pub fn run(self, keys: Keys) -> Result<()> {
        keys.print(&self.key_name, self.private)?;

        Ok(())
    }
}
