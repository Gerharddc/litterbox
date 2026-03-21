use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use inquire_derive::Selectable;
use log::info;
use nix::unistd::{Gid, Uid};
use std::{ffi::OsString, fmt::Display, path::PathBuf, process::Output};
use tabled::{Table, Tabled};

mod agent;
mod daemon;
mod devices;
mod env;
mod files;
mod keys;
mod podman;
mod sandbox;
mod settings;

use crate::{
    agent::prompt_confirmation,
    devices::attach_device,
    files::{dockerfile_path, wait_for_sessions_to_finish, write_file},
    keys::Keys,
    podman::*,
    sandbox::entrypoint,
};

#[derive(Tabled)]
struct ContainerTableRow {
    name: String,
    container_id: String,
    container_names: String,
    image: String,
    image_id: String,
}

impl From<&ContainerDetails> for ContainerTableRow {
    fn from(value: &ContainerDetails) -> Self {
        Self {
            name: value.labels.name.clone(),
            container_id: value.id.chars().take(12).collect(),
            container_names: value.names.join(","),
            image: value.image.clone(),
            image_id: value.image_id.chars().take(12).collect(),
        }
    }
}

fn extract_stdout(output: &Output) -> Result<&str> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        bail!("Podman command failed: {stderr}");
    }

    Ok(str::from_utf8(&output.stdout)?)
}

#[derive(Debug, Copy, Clone, Selectable)]
enum Template {
    OpenSuseTumbleweed,
    UbuntuLts,
    CachyOS,
}

impl Template {
    fn contents(&self) -> &'static str {
        match self {
            Template::OpenSuseTumbleweed => include_str!("../templates/tumbleweed.Dockerfile"),
            Template::UbuntuLts => include_str!("../templates/ubuntu-latest.Dockerfile"),
            Template::CachyOS => include_str!("../templates/cachyos.Dockerfile"),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Template::OpenSuseTumbleweed => "OpenSUSE Tumbleweed",
            Template::UbuntuLts => "Ubuntu LTS",
            Template::CachyOS => "CachyOS",
        }
    }
}

impl Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

fn define_litterbox(lbx_name: &str) -> Result<()> {
    let dockerfile = dockerfile_path(lbx_name)?;

    if dockerfile.exists() {
        bail!("Dockerfile already exists at {dockerfile:?}");
    }

    let template = Template::select("Choose a template:").prompt()?;

    write_file(dockerfile.as_path(), template.contents())?;
    info!("Default Dockerfile written to {dockerfile:?}");

    Ok(())
}

fn gen_random_name() -> String {
    let mut generator = names::Generator::with_naming(names::Name::Numbered);
    let name = generator.next().expect("Name should not be none.");

    format!("lbx-{name}")
}

/// Simple sandbox utility aimed at software development
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Define a new Litterbox using a template Dockerfile
    #[clap(visible_alias("def"))]
    Define {
        /// The name of the Litterbox to define
        name: String,
    },

    /// Build a new Litterbox
    Build {
        /// The name of the Litterbox to build
        name: String,
    },

    /// List all the Litterboxes that have been created
    #[clap(visible_alias("ls"))]
    List,

    /// Enter an existing Litterbox
    Enter {
        /// The name of the Litterbox to enter
        name: String,

        /// Make STDIN available to the contained process. Defaults to "true" if
        /// COMMAND is not supplied
        #[arg(long, short, default_value_t = false)]
        interactive: bool,

        /// Allocate a pseudo-TTY. Defaults to "true" if COMMAND is not supplied
        #[arg(long, short, default_value_t = false)]
        tty: bool,

        /// Working directory inside the container
        #[arg(long, short)]
        workdir: Option<PathBuf>,

        /// Run as root inside the container
        #[arg(long, default_value_t = false)]
        root: bool,

        /// The command to execute instead of the login shell
        command: Option<OsString>,

        /// Additional arguments passed to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },

    /// Delete an existing Litterbox
    #[clap(visible_alias("del"), visible_alias("rm"))]
    Delete {
        /// The name of the Litterbox to delete
        name: String,
    },

    /// Manage SSH keys that can be exposed to Litterboxes
    #[command(subcommand)]
    Keys(KeysCommand),

    /// Attach a device to a Litterbox (the device fille be created in the home directory)
    #[clap(visible_alias("dev"))]
    Device {
        /// The name of the Litterbox to attach the device to
        name: String,

        /// The path of the device to be attached
        path: String,
    },

    /// Ask the user to confirm a request (for internal use)
    #[clap(hide = true)]
    Confirm {
        // The request that the user needs to confirm
        #[arg(long)]
        request: String,

        // The name of the litterbox sending the request
        #[arg(long)]
        lbx_name: String,
    },

    /// Run daemon (for internal use)
    #[clap(hide = true)]
    Daemon {
        /// The name of the Litterbox
        name: String,
    },

    /// Wait for the Litterbox to finish (for internal use)
    #[clap(hide = true)]
    Wait,

    /// Container entrypoint (for internal use)
    // -h and -V might conflict with a command's arguments
    #[clap(hide = true, disable_help_flag = true, disable_version_flag = true)]
    Entrypoint {
        /// Run as root instead of dropping privileges
        #[arg(long, default_value_t = false)]
        root: bool,

        /// The UID to drop to if dropping privileges
        #[arg(long, value_parser = |x: &str| x.parse().map(Uid::from_raw))]
        uid: Uid,

        /// The GID to drop to if dropping privileges
        #[arg(long, value_parser = |x: &str| x.parse().map(Gid::from_raw))]
        gid: Gid,

        /// The command to execute instead of the login shell
        command: Option<OsString>,

        /// Additional arguments passed to the command
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
}

impl Command {
    fn run(self) -> Result<()> {
        match self {
            Self::Confirm { request, lbx_name } => prompt_confirmation(&request, &lbx_name),

            Self::Define { name } => define_litterbox(&name)?,

            Self::Delete { name } => delete_litterbox(&name)?,

            Self::Keys(cmd) => cmd.run()?,

            Self::Wait => wait_for_sessions_to_finish()?,

            Self::Enter {
                name,
                interactive,
                tty,
                workdir,
                command,
                args,
                root,
            } => enter_litterbox(&name, interactive, tty, workdir, command, args, root)?,

            Self::Build { name } => {
                build_image(&name)?;
                build_litterbox(&name)?;
            }

            Self::List => {
                let containers = list_containers()?;
                let table_rows: Vec<ContainerTableRow> =
                    containers.0.iter().map(|c| c.into()).collect();
                let table = Table::new(table_rows);

                println!("{table}");
            }

            Self::Device { name, path } => {
                let dest_path = attach_device(&name, &path)?;

                println!("Device attached at {dest_path:#?}!");
            }

            Self::Daemon { name } => {
                use std::io::{Read, stdin};

                let mut password = String::new();
                stdin().read_to_string(&mut password)?;
                let password = password.trim();

                // We wait to create the runtime here since only this one command depends on it.
                let rt = tokio::runtime::Runtime::new().expect("Tokio runtime should start");
                rt.block_on(daemon::run(&name, password))?;
            }

            Self::Entrypoint {
                root,
                uid,
                gid,
                command,
                args,
            } => entrypoint(root, uid, gid, command, args)?,
        }

        Ok(())
    }
}

#[derive(Subcommand, Debug)]
enum KeysCommand {
    /// List all the keys are being managed
    #[clap(visible_alias("ls"))]
    List,

    /// Generate a new random key
    Generate {
        /// The name of the key
        name: String,
    },

    /// Import a key to Litterbox
    Import {
        /// The name of the new key
        name: String,
        /// The file path to the key
        path: PathBuf,
    },

    /// Delete an existing key
    Delete {
        /// The name of the key
        name: String,
    },

    /// Attach an existing key to a Litterbox
    Attach {
        /// The name of the key
        key_name: String,

        /// The name of the Litterbox
        litterbox_name: String,
    },

    /// Detach an attached Litterbox from a key
    Detach {
        /// The name of the key
        key_name: String,
    },

    /// Print the key in OpenSSH public key format
    Print {
        /// The name of the key
        key_name: String,

        /// Print the private key instead of the public key
        #[clap(long)]
        private: bool,
    },

    /// Change the password used to encrypt passwords for storage
    ChangePassword {},
}

impl KeysCommand {
    fn run(self) -> Result<()> {
        let mut keys = Keys::load()?;

        match self {
            Self::Attach {
                key_name,
                litterbox_name,
            } => keys.attach(&key_name, &litterbox_name)?,

            Self::ChangePassword {} => keys.change_password()?,

            Self::Delete { name } => keys.delete(&name)?,

            Self::Detach { key_name } => keys.detach(&key_name)?,

            Self::Generate { name } => keys.generate(&name)?,

            Self::Import { name, path } => keys.import_key(&name, path)?,

            Self::List => keys.print_list(),

            Self::Print { key_name, private } => keys.print(&key_name, private)?,
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    env_logger::init();

    let argv_0 = std::env::args().next();
    if matches!(argv_0.as_deref(), Some("run0" | "sudo")) {
        eprintln!(
            "run0/sudo is not supported inside this session. Use 'litterbox enter --root <name>' to enter as root."
        );

        return Ok(());
    }

    let args = Args::parse();

    args.command.run()
}
