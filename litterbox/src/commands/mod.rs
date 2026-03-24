use clap::Subcommand;

mod build;
mod confirm;
mod daemon;
mod define;
mod delete;
mod device;
mod enter;
mod entrypoint;
mod keys;
mod list;
mod wait;

#[derive(Subcommand, Debug)]
pub enum Command {
    Build(#[clap(flatten)] build::Command),

    #[clap(visible_alias("def"))]
    Define(#[clap(flatten)] define::Command),

    #[clap(visible_alias("del"), visible_alias("rm"))]
    Delete(#[clap(flatten)] delete::Command),

    #[clap(visible_alias("dev"))]
    Device(#[clap(flatten)] device::Command),

    Enter(#[clap(flatten)] enter::Command),

    #[clap(visible_alias("ls"))]
    List(#[clap(flatten)] list::Command),

    #[command(subcommand)]
    Keys(keys::Command),

    #[clap(hide = true)]
    Confirm(#[clap(flatten)] confirm::Command),

    #[clap(hide = true)]
    Daemon(#[clap(flatten)] daemon::Command),

    #[clap(hide = true)]
    Wait(#[clap(flatten)] wait::Command),

    // -h and -V conflict with a command's arguments
    #[clap(hide = true, disable_help_flag = true, disable_version_flag = true)]
    Entrypoint(#[clap(flatten)] entrypoint::Command),
}

impl Command {
    pub fn run(self) -> anyhow::Result<()> {
        match self {
            Command::Define(command) => command.run(),
            Command::Build(command) => command.run(),
            Command::List(command) => command.run(),
            Command::Enter(command) => command.run(),
            Command::Delete(command) => command.run(),
            Command::Keys(command) => command.run(),
            Command::Device(command) => command.run(),
            Command::Confirm(command) => command.run(),
            Command::Daemon(command) => command.run(),
            Command::Wait(command) => command.run(),
            Command::Entrypoint(command) => command.run(),
        }
    }
}
