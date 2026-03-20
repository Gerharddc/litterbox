use crate::keys::Keys;
use anyhow::Result;
use clap::Subcommand;

pub mod attach;
pub mod change_password;
pub mod delete;
pub mod detach;
pub mod generate;
pub mod import;
pub mod list;
pub mod print;

/// Manage SSH keys that can be exposed to Litterboxes
#[derive(Subcommand, Debug)]
pub enum Command {
    Attach(#[clap(flatten)] attach::Command),

    ChangePassword(#[clap(flatten)] change_password::Command),

    Delete(#[clap(flatten)] delete::Command),

    Detach(#[clap(flatten)] detach::Command),

    Generate(#[clap(flatten)] generate::Command),

    Import(#[clap(flatten)] import::Command),

    #[clap(visible_alias("ls"))]
    List(#[clap(flatten)] list::Command),

    Print(#[clap(flatten)] print::Command),
}

impl Command {
    pub fn run(self) -> Result<()> {
        let keys = Keys::load()?;

        match self {
            Command::List(command) => command.run(keys),
            Command::Generate(command) => command.run(keys),
            Command::Import(command) => command.run(keys),
            Command::Delete(command) => command.run(keys),
            Command::Attach(command) => command.run(keys),
            Command::Detach(command) => command.run(keys),
            Command::Print(command) => command.run(keys),
            Command::ChangePassword(command) => command.run(keys),
        }
    }
}
