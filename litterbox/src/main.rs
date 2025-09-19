use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{
    env,
    ffi::OsString,
    fs, io,
    path::Path,
    process::{Command, ExitStatus, Output},
};

#[derive(Deserialize, Debug)]
struct LitterboxLabels {
    #[serde(rename = "io.litterbox.name")]
    name: String,
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
    RunPodman(io::Error),
    PodmanError(ExitStatus, String),
    ParseOutput(std::str::Utf8Error),
    Deserialize(serde_json::error::Error),
    EnvVarUndefined(&'static str),
    EnvVarInvalid(&'static str, OsString),
    DirUncreatable(io::Error, String),
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
                eprintln!("error code: {:#?}, message: {}", exit_status, stderr);
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
                println!("Environment variable not defined: {}.", name)
            }
            LitterboxError::EnvVarInvalid(name, value) => {
                println!("Environment variable not a valid string: {}.", name);

                // TODO: use env_logger instead
                eprintln!("{:#?}", value);
            }
            LitterboxError::DirUncreatable(error, dir) => {
                println!("Directory could not be created: {}.", dir);

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
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
            "label=io.litterbox.name",
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    let stdout = extract_stdout(&output)?;
    Ok(serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)?)
}

fn get_env(name: &'static str) -> Result<String, LitterboxError> {
    env::var_os(name)
        .ok_or(LitterboxError::EnvVarUndefined(name))?
        .into_string()
        .map_err(|value| LitterboxError::EnvVarInvalid(name, value))
}

fn path_relative_to_home(relative_path: &str) -> Result<String, LitterboxError> {
    let home_dir = get_env("HOME")?;
    let home_path = Path::new(&home_dir);
    let full_path = home_path.join(relative_path);

    // TODO: maybe don't do the lossy conversion?
    Ok(full_path.to_string_lossy().to_string())
}

fn build_container(name: &str, password: &str) -> Result<(), LitterboxError> {
    let dockerfile_path = path_relative_to_home(&format!("Litterbox/{}.Dockerfile", name))?;
    let name_label = format!("io.litterbox.name={}", name);
    let password_arg = format!("PASSWORD={}", password);
    let image_name = format!("litterbox-{}", name);

    let output = Command::new("podman")
        .args([
            "build",
            "--build-arg",
            &password_arg,
            "-t",
            &image_name,
            "--label",
            &name_label,
            "-f",
            &dockerfile_path,
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    let stdout = extract_stdout(&output)?;
    println!("stdout: {}", stdout);

    Ok(())
}

fn create_litterbox(name: &str) -> Result<(), LitterboxError> {
    let name_label = format!("io.litterbox.name={}", name);
    let image_name = format!("litterbox-{}", name);

    let wayland_display = get_env("WAYLAND_DISPLAY")?;
    let xdg_runtime_dir = get_env("XDG_RUNTIME_DIR")?;

    let litterbox_home = path_relative_to_home(&format!("Litterbox/{}", name))?;
    fs::create_dir_all(&litterbox_home)
        .map_err(|e| LitterboxError::DirUncreatable(e, litterbox_home.clone()))?;

    let output = Command::new("podman")
        .args([
            "create",
            "--name",
            &image_name,
            "--userns=keep-id",
            "--device",
            "/dev/dri",
            "--hostname",
            "litterbox",                    // TODO: think if we want to change this
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
            &format!("{litterbox_home}:/home/user"),
            "--label",
            &name_label,
            &image_name,
        ])
        .output()
        .map_err(LitterboxError::RunPodman)?;

    let stdout = extract_stdout(&output)?;
    println!("stdout: {}", stdout);

    Ok(())
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
            create_litterbox(&name)?;
            println!("Litterbox created!");
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
