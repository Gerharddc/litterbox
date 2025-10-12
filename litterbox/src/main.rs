use clap::{Parser, Subcommand};
use inquire::{Confirm, InquireError, Password};
use inquire_derive::Selectable;
use serde::Deserialize;
use std::{
    env,
    ffi::OsString,
    fmt::Display,
    fs, io,
    path::Path,
    process::{Command, ExitStatus, Output},
};
use tabled::{Table, Tabled};

use crate::files::{dockerfile_path, path_relative_to_home};

mod files;
mod keys;

#[derive(Deserialize, Debug)]
struct LitterboxLabels {
    #[serde(rename = "io.litterbox.name")]
    name: String,
}

#[derive(Deserialize, Debug)]
struct ContainerDetails {
    #[serde(rename = "Id")]
    id: String,

    #[serde(rename = "Image")]
    image: String,

    #[serde(rename = "ImageID")]
    image_id: String,

    #[serde(rename = "Names")]
    names: Vec<String>,

    #[serde(rename = "Labels")]
    labels: LitterboxLabels,
}

#[derive(Deserialize, Debug)]
struct AllContainers(Vec<ContainerDetails>);

#[derive(Deserialize, Debug)]
struct ImageDetails {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Deserialize, Debug)]
struct AllImages(Vec<ImageDetails>);

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

#[derive(Debug)]
enum LitterboxError {
    RunPodman(io::Error),
    PodmanError(ExitStatus, String),
    ParseOutput(std::str::Utf8Error),
    Deserialize(serde_json::error::Error),
    EnvVarUndefined(&'static str),
    EnvVarInvalid(&'static str, OsString),
    DirUncreatable(io::Error, String),
    WriteFailed(io::Error, String),
    ReadFailed(io::Error, String),
    NoContainerForName,
    MultipleContainersForName,
    ContainerAlreadyExists(String),
    NoImageForName,
    MultipleImagesForName,
    ImageAlreadyExists(String),
    DockerfileAlreadyExists(String),
    PromptError(InquireError),
}

impl LitterboxError {
    pub fn print(&self) {
        match self {
            LitterboxError::RunPodman(e) => {
                println!("Could not run podman command. Perhaps it is not installed?");

                // TODO: use env_logger instead
                eprintln!("{:#?}", e);
            }
            LitterboxError::PodmanError(exit_status, stderr) => {
                println!("Podman command returned non-zero error code.");

                // TODO: use env_logger instead
                eprintln!("error code: {:#?}, message: {stderr}", exit_status);
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
            LitterboxError::EnvVarUndefined(name) => {
                println!("Environment variable not defined: {name}.")
            }
            LitterboxError::EnvVarInvalid(name, value) => {
                println!("Environment variable not a valid string: {name}.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", value);
            }
            LitterboxError::DirUncreatable(error, dir) => {
                println!("Directory could not be created: {dir}.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
            }
            LitterboxError::WriteFailed(error, path) => {
                println!("File could not be written: {path}.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
            }
            LitterboxError::ReadFailed(error, path) => {
                println!("File could not be read: {path}.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
            }
            LitterboxError::NoContainerForName => {
                println!("A container with the specified Litterbox name could not be found.");
            }
            LitterboxError::MultipleContainersForName => {
                println!("Multiple containers were found with the specified Litterbox name.");
            }
            LitterboxError::ContainerAlreadyExists(id) => {
                println!("Container for Litterbox already exists with id: {id}.");
            }
            LitterboxError::NoImageForName => {
                println!("An image with the specified Litterbox name could not be found.");
            }
            LitterboxError::MultipleImagesForName => {
                println!("Multiple images were found with the specified Litterbox name.");
            }
            LitterboxError::ImageAlreadyExists(id) => {
                println!("Image for Litterbox already exists with id: {id}.");
            }
            LitterboxError::DockerfileAlreadyExists(path) => {
                println!("Dockerfile for Litterbox already exists at {path}.");
            }
            LitterboxError::PromptError(error) => {
                println!("Failed to retrieve valid input from user.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
            }
        }
    }
}

fn get_env(lbx_name: &'static str) -> Result<String, LitterboxError> {
    env::var_os(lbx_name)
        .ok_or(LitterboxError::EnvVarUndefined(lbx_name))?
        .into_string()
        .map_err(|value| LitterboxError::EnvVarInvalid(lbx_name, value))
}

fn extract_stdout(output: &Output) -> Result<&str, LitterboxError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // TODO: perhaps we can just store the COW instead?
        return Err(LitterboxError::PodmanError(
            output.status,
            stderr.into_owned(),
        ));
    }

    str::from_utf8(&output.stdout).map_err(LitterboxError::ParseOutput)
}

fn list_containers() -> Result<AllContainers, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "ps",
            "-a",
            "--format",
            "json",
            "--filter",
            "label=io.litterbox.name",
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    let stdout = extract_stdout(&output)?;
    serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)
}

fn get_container_id(lbx_name: &str) -> Result<String, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "ps",
            "-a",
            "--format",
            "json",
            "--filter",
            &format!("label=io.litterbox.name={lbx_name}"),
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    let stdout = extract_stdout(&output)?;
    let containers: AllContainers =
        serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)?;

    match containers.0.len() {
        0 => Err(LitterboxError::NoContainerForName),
        1 => Ok(containers.0[0].id.clone()),
        _ => Err(LitterboxError::MultipleContainersForName),
    }
}

#[derive(Debug, Copy, Clone, Selectable)]
enum Template {
    OpenSuseTumbleweed,
    UbuntuLts,
}

impl Template {
    fn contents(&self) -> &'static str {
        match self {
            Template::OpenSuseTumbleweed => include_str!("tumbleweed.Dockerfile"),
            Template::UbuntuLts => include_str!("ubuntu-latest.Dockerfile"),
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Template::OpenSuseTumbleweed => "OpenSUSE Tumbleweed",
            Template::UbuntuLts => "Ubuntu LTS",
        }
    }
}

impl Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

fn prepare_litterbox(lbx_name: &str) -> Result<(), LitterboxError> {
    let dockerfile_path = dockerfile_path(lbx_name)?;
    if Path::new(&dockerfile_path).exists() {
        return Err(LitterboxError::DockerfileAlreadyExists(dockerfile_path));
    }

    // TODO: should we really expect here?
    let template = Template::select("Choose a template:")
        .prompt()
        .expect("Unexpected error selecting template.");

    let output_dir = Path::new(&dockerfile_path).parent().unwrap();
    fs::create_dir_all(output_dir)
        .map_err(|e| LitterboxError::DirUncreatable(e, output_dir.to_string_lossy().into()))?;

    fs::write(&dockerfile_path, template.contents())
        .map_err(|e| LitterboxError::WriteFailed(e, dockerfile_path.to_owned()))?;

    println!("Default Dockerfile written to {dockerfile_path}");
    Ok(())
}

fn gen_random_name() -> String {
    let mut generator = names::Generator::with_naming(names::Name::Numbered);

    // TODO: is it really safe to unwrap here?
    let name = generator.next().unwrap();

    format!("lbx-{name}")
}

fn get_image_id(lbx_name: &str) -> Result<String, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "image",
            "ls",
            "-a",
            "--format",
            "json",
            "--filter",
            &format!("label=io.litterbox.name={lbx_name}"),
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    let stdout = extract_stdout(&output)?;
    let images: AllImages = serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)?;

    match images.0.len() {
        0 => Err(LitterboxError::NoImageForName),
        1 => Ok(images.0[0].id.clone()),
        _ => Err(LitterboxError::MultipleImagesForName),
    }
}

fn build_image(lbx_name: &str, user: &str) -> Result<(), LitterboxError> {
    match get_image_id(lbx_name) {
        Ok(id) => return Err(LitterboxError::ImageAlreadyExists(id)),
        Err(LitterboxError::NoImageForName) => {}
        Err(other) => return Err(other),
    };

    let dockerfile_path = dockerfile_path(lbx_name)?;
    if !Path::new(&dockerfile_path).exists() {
        println!("{dockerfile_path} does not exist. Please make one or a use a provided template.");
        prepare_litterbox(lbx_name)?;
    }

    let password = Password::new("Password:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()
        .map_err(LitterboxError::PromptError)?;

    let image_name = gen_random_name();
    let mut child = Command::new("podman")
        .args([
            "build",
            "--build-arg",
            &format!("USER={}", user),
            "--build-arg",
            &format!("PASSWORD={}", password),
            "-t",
            &image_name,
            "--label",
            &format!("io.litterbox.name={lbx_name}"),
            "-f",
            &dockerfile_path,
        ])
        .spawn()
        .map_err(LitterboxError::RunPodman)?;

    child.wait().map_err(LitterboxError::RunPodman)?;
    println!("Built image named {image_name}.");
    Ok(())
}

fn create_litterbox(lbx_name: &str, user: &str) -> Result<(), LitterboxError> {
    match get_container_id(lbx_name) {
        Ok(id) => return Err(LitterboxError::ContainerAlreadyExists(id)),
        Err(LitterboxError::NoContainerForName) => {}
        Err(other) => return Err(other),
    };

    let image_id = get_image_id(lbx_name)?;
    let container_name = gen_random_name();

    let wayland_display = get_env("WAYLAND_DISPLAY")?;
    let xdg_runtime_dir = get_env("XDG_RUNTIME_DIR")?;

    let litterbox_home = path_relative_to_home(lbx_name)?;
    fs::create_dir_all(&litterbox_home)
        .map_err(|e| LitterboxError::DirUncreatable(e, litterbox_home.clone()))?;

    let mut child = Command::new("podman")
        .args([
            "create",
            "--tty",
            "--name",
            &container_name,
            "--userns=keep-id",
            "--device",
            "/dev/dri",
            "--hostname",
            &format!("lbx-{lbx_name}"),
            "--security-opt=label=disable", // FIXME: use udica to make better rules instead
            "-e",
            &format!("WAYLAND_DISPLAY={wayland_display}"),
            "-e",
            "XDG_RUNTIME_DIR=/tmp",
            "-v",
            &format!("{xdg_runtime_dir}/{wayland_display}:/tmp/{wayland_display}"),
            "-v",
            "/dev/dri:/dev/dri",
            "-v",
            &format!("{litterbox_home}:/home/{user}"),
            "--label",
            &format!("io.litterbox.name={lbx_name}"),
            &image_id,
        ])
        .spawn()
        .map_err(LitterboxError::RunPodman)?;

    child.wait().map_err(LitterboxError::RunPodman)?;
    println!("Created container named {container_name}.");
    Ok(())
}

fn enter_distrobox(name: &str) -> Result<(), LitterboxError> {
    let mut child = Command::new("podman")
        .args([
            "start",
            "--interactive",
            "--attach",
            &get_container_id(name)?,
        ])
        .spawn()
        .map_err(LitterboxError::RunPodman)?;

    child.wait().map_err(LitterboxError::RunPodman)?;
    Ok(())
}

fn delete_distrobox(lbx_name: &str) -> Result<(), LitterboxError> {
    // We check if it exists before promting the user
    let container_id = get_container_id(lbx_name)?;

    let should_delete = Confirm::new("Are you sure you want to delete this Litterbox?")
        .with_default(false)
        .with_help_message(
            "This operation cannot be undone and will delete all data/state outside the home directory.",
        )
        .prompt();

    match should_delete {
        Ok(true) => {}
        _ => {
            println!("Okay, the Litterbox won't be deleted!");
            return Ok(());
        }
    }

    let mut child = Command::new("podman")
        .args(["rm", &container_id])
        .spawn()
        .map_err(LitterboxError::RunPodman)?;

    child.wait().map_err(LitterboxError::RunPodman)?;
    println!("Container for Litterbox deleted!");

    let image_id = get_image_id(lbx_name)?;

    let mut child = Command::new("podman")
        .args(["image", "rm", &image_id])
        .spawn()
        .map_err(LitterboxError::RunPodman)?;

    child.wait().map_err(LitterboxError::RunPodman)?;
    println!("Image for Litterbox deleted!");

    // TODO: ask the user if they also want the home dir deleted
    Ok(())
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
    /// Prepare a new Litterbox with a template Dockerfile
    #[clap(visible_alias("prep"))]
    Prepare {
        /// The name of the Litterbox to prepare
        name: String,
    },

    /// Create a new Litterbox
    Create {
        /// The name of the Litterbox to create
        name: String,

        /// The username of the user in the Litterbox (defaults to "user")
        #[arg(short, long)]
        user: Option<String>,
    },

    /// List all the Litterboxes that have been created
    #[clap(visible_alias("ls"))]
    List,

    /// Enter an existing Litterbox
    Enter {
        /// The name of the Litterbox to enter
        name: String,
    },

    /// Delete an existing Litterbox.
    #[clap(visible_alias("del"), visible_alias("rm"))]
    Delete {
        /// The name of the Litterbox to delete
        name: String,
    },

    /// Manage SSH keys that can be exposed to Litterboxes
    #[command(subcommand)]
    Keys(KeyCommands),
}

#[derive(Subcommand, Debug)]
enum KeyCommands {
    /// List all the keys are being managed
    #[clap(visible_alias("ls"))]
    List,

    /// Generate a new random key
    Generate {
        /// The name of the key
        name: String,
    },

    /// Delete an existing key
    Delete {
        /// The name of the key
        name: String,
    },
}

fn try_run() -> Result<(), LitterboxError> {
    let args = Args::parse();

    match args.command {
        Commands::Prepare { name } => {
            prepare_litterbox(&name)?;
            println!("Litterbox prepared!");
        }
        Commands::Create { name, user } => {
            let user = user.unwrap_or("user".to_string());
            build_image(&name, &user)?;
            create_litterbox(&name, &user)?;
            println!("Litterbox created!");
        }
        Commands::Enter { name } => {
            enter_distrobox(&name)?;
            println!("Exited Litterbox...")
        }
        Commands::List => {
            let containers = list_containers()?;
            let table_rows: Vec<ContainerTableRow> =
                containers.0.iter().map(|c| c.into()).collect();
            let table = Table::new(table_rows);
            println!("{table}");
        }
        Commands::Delete { name } => {
            delete_distrobox(&name)?;
        }
        Commands::Keys(cmd) => process_key_cmd(cmd)?,
    }

    Ok(())
}

fn process_key_cmd(cmd: KeyCommands) -> Result<(), LitterboxError> {
    match cmd {
        KeyCommands::List => todo!(),
        KeyCommands::Generate { name } => todo!(),
        KeyCommands::Delete { name } => todo!(),
    }

    Ok(())
}

fn main() {
    if let Err(e) = try_run() {
        e.print();
    }
}
