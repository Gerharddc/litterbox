use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Output},
};

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
enum LitterboxError {
    RunPodman(std::io::Error),
    PodmanError(ExitStatus, String),
    ParseOutput(std::str::Utf8Error),
    Deserialize(serde_json::error::Error),
    HomeNotDefined,
}

impl LitterboxError {
    pub fn print(&self) {
        match self {
            LitterboxError::RunPodman(e) => {
                println!("Could not run podman command. Perhaps it is not installed?");

                // TODO: use env_logger instead
                eprintln!("{:#?}", e);
            }
            LitterboxError::ParseOutput(e) => {
                println!("Could not parse output from podman.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", e);
            }
            LitterboxError::Deserialize(e) => {
                println!("Could not deserialize output from podman. Unexpected format.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", e);
            }
            LitterboxError::HomeNotDefined => {
                println!("Home directory not defined in environment variables.")
            }
            LitterboxError::PodmanError(exit_status, stderr) => {
                println!("Podman command returned non-zero error code.");

                // TODO: use env_logger instead
                eprintln!("error code: {:#?}, message: {}", exit_status, stderr);
            }
        }
    }
}

fn extract_stdout(output: &Output) -> Result<&str, LitterboxError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // TODO: perhaps we can just store the COW instead
        return Err(LitterboxError::PodmanError(
            output.status,
            stderr.into_owned(),
        ));
    }

    Ok(str::from_utf8(&output.stdout).map_err(LitterboxError::ParseOutput)?)
}

fn list_containers() -> Result<AllContainers, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "ps",
            "-a",
            "--format",
            "json",
            "--filter",
            "label=org.opensuse.distrobox.title",
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    let stdout = extract_stdout(&output)?;
    Ok(serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)?)
}

fn path_relative_to_home(relative_path: &str) -> Result<PathBuf, LitterboxError> {
    let home_dir = env::var_os("HOME").ok_or(LitterboxError::HomeNotDefined)?;
    let home_path = Path::new(&home_dir);
    Ok(home_path.join(relative_path))
}

fn build_container(name: &str, password: &str) -> Result<String, LitterboxError> {
    let password_arg = format!("PASSWORD={}", password);
    let dockerfile_path = path_relative_to_home(&format!("Litterbox/{}.Dockerfile", name))?;

    let output = Command::new("podman")
        .args([
            "build",
            "--quiet",
            "--build-arg",
            &password_arg,
            "-t",
            name,
            "-f",
            &dockerfile_path.to_string_lossy(),
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    // Image ID will be printed to stdout
    let stdout = extract_stdout(&output)?;
    let image_id = stdout.trim(); // should be sha256:... or image ID
    Ok(image_id.to_string())
}

fn create_litterbox(_name: &str) {
    todo!()
}

fn enter_distrobox(_name: &str) {
    todo!()
}

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

fn try_run() -> Result<(), LitterboxError> {
    let args = Args::parse();

    match args.command {
        Commands::Create { name, password } => {
            build_container(&name, &password)?;
            create_litterbox(&name);
            println!("Container created!");
        }
        Commands::Enter { name } => {
            enter_distrobox(&name);
        }
        Commands::List => {
            let containers = list_containers()?;
            println!("containers: {:#?}", containers);
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = try_run() {
        e.print();
    }
}
