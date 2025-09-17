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

fn list_containers() -> Result<AllContainers, std::io::Error> {
    let res = Command::new("podman")
        .args([
            "ps",
            "-a",
            "--format",
            "json",
            "--filter",
            "label=org.opensuse.distrobox.title",
        ])
        .output()?;

    // FIXME: do not unwrap
    let output = std::str::from_utf8(&res.stdout).unwrap();
    let containers: AllContainers = serde_json::from_str(output).unwrap();
    Ok(containers)
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
        Commands::List => {
            let containers = list_containers().unwrap();
            println!("containers: {:#?}", containers);
        }
    }
}
