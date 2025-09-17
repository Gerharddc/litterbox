use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::process::Command;

/// Simple sandbox utility aimed at software development.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Creates a new litterbox
    Create {
        /// The name of the litterbox
        #[arg(short, long)]
        name: String,

        /// The password of the user in the litterbox
        #[arg(short, long)]
        password: String,
    },

    /// Enters an existing litterbox
    Enter {
        /// The name of the litterbox
        #[arg(short, long)]
        name: String,
    },

    /// Lists all the litterboxes that have been created
    List,
}

#[derive(Deserialize, Debug)]
struct LitterboxLabels {
    #[serde(rename = "org.opensuse.distrobox.title")]
    title: String,
}

#[derive(Deserialize, Debug)]
struct ContainerDetails {
    #[serde(rename = "Names")]
    names: Vec<String>,

    #[serde(rename = "Labels")]
    labels: LitterboxLabels,
}

#[derive(Deserialize, Debug)]
struct AllContainers(Vec<ContainerDetails>);

#[derive(Debug)]
enum ListContainerError {
    RunPodman(std::io::Error),
    ParseOutput(std::str::Utf8Error),
    Deserialize(serde_json::error::Error),
}

impl ListContainerError {
    pub fn print(&self) {
        match self {
            ListContainerError::RunPodman(e) => {
                println!("Could not run podman command. Perhaps it is not installed?");

                // TODO: use env_logger instead
                eprintln!("{:#?}", e);
            }
            ListContainerError::ParseOutput(e) => {
                println!("Could not parse output from podman.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", e);
            }
            ListContainerError::Deserialize(e) => {
                println!("Could not deserialize output from podman. Unexpected format.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", e);
            }
        }
    }
}

fn list_containers() -> Result<AllContainers, ListContainerError> {
    let res = Command::new("podman")
        .args([
            "ps",
            "-a",
            "--format",
            "json",
            "--filter",
            "label=org.opensuse.distrobox.title",
        ])
        .output()
        .map_err(ListContainerError::RunPodman)?;

    let output = std::str::from_utf8(&res.stdout).map_err(ListContainerError::ParseOutput)?;

    Ok(serde_json::from_str(output).map_err(ListContainerError::Deserialize)?)
}

fn create_litterbox(_name: &str, _password: &str) {
    todo!()
}

fn enter_distrobox(_name: &str) {
    todo!()
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Create { name, password } => {
            create_litterbox(&name, &password);
        }
        Commands::Enter { name } => {
            enter_distrobox(&name);
        }
        Commands::List => match list_containers() {
            Ok(containers) => println!("containers: {:#?}", containers),
            Err(e) => e.print(),
        },
    }
}
