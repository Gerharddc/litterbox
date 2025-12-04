use inquire::{Confirm, Password};
use inquire_derive::Selectable;
use log::{debug, info};
use serde::Deserialize;
use std::{
    fmt::Display,
    fs,
    process::{Child, Command},
};

use crate::{
    define_litterbox,
    errors::LitterboxError,
    extract_stdout,
    files::{SshSockFile, dockerfile_path, lbx_home_path},
    gen_random_name, get_env,
};

#[derive(Deserialize, Debug)]
pub struct LitterboxLabels {
    #[serde(rename = "work.litterbox.name")]
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct ContainerDetails {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "Image")]
    pub image: String,

    #[serde(rename = "ImageID")]
    pub image_id: String,

    #[serde(rename = "Names")]
    pub names: Vec<String>,

    #[serde(rename = "Labels")]
    pub labels: LitterboxLabels,
}

#[derive(Deserialize, Debug)]
pub struct AllContainers(pub Vec<ContainerDetails>);

#[derive(Deserialize, Debug)]
pub struct ImageDetails {
    #[serde(rename = "Id")]
    pub id: String,
}

#[derive(Deserialize, Debug)]
struct AllImages(Vec<ImageDetails>);

pub fn list_containers() -> Result<AllContainers, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "ps",
            "-a",
            "--format",
            "json",
            "--filter",
            "label=work.litterbox.name",
        ])
        .output()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    let stdout = extract_stdout(&output)?;
    serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)
}

pub fn get_container_id(lbx_name: &str) -> Result<String, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "ps",
            "-a",
            "--format",
            "json",
            "--filter",
            &format!("label=work.litterbox.name={lbx_name}"),
        ])
        .output()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    let stdout = extract_stdout(&output)?;
    let containers: AllContainers =
        serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)?;

    match containers.0.len() {
        0 => Err(LitterboxError::NoContainerForName),
        1 => Ok(containers.0[0].id.clone()),
        _ => Err(LitterboxError::MultipleContainersForName),
    }
}

pub fn get_image_id(lbx_name: &str) -> Result<String, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "image",
            "ls",
            "-a",
            "--format",
            "json",
            "--filter",
            &format!("label=work.litterbox.name={lbx_name}"),
        ])
        .output()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    let stdout = extract_stdout(&output)?;
    let images: AllImages = serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)?;

    match images.0.len() {
        0 => Err(LitterboxError::NoImageForName),
        1 => Ok(images.0[0].id.clone()),
        _ => Err(LitterboxError::MultipleImagesForName),
    }
}

fn wait_for_podman(mut child: Child) -> Result<(), LitterboxError> {
    let res = child
        .wait()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    if !res.success() {
        Err(LitterboxError::CommandFailed(res, "podman"))
    } else {
        Ok(())
    }
}

pub fn build_image(lbx_name: &str, user: &str) -> Result<(), LitterboxError> {
    match get_image_id(lbx_name) {
        Ok(id) => return Err(LitterboxError::ImageAlreadyExists(id)), // TODO: instead prompt user how to proceed
        Err(LitterboxError::NoImageForName) => {}
        Err(other) => return Err(other),
    };

    let dockerfile_path = dockerfile_path(lbx_name)?;
    if !dockerfile_path.exists() {
        println!(
            "{} does not exist. Please make one or a use a provided template.",
            dockerfile_path.display()
        );
        define_litterbox(lbx_name)?;
    }

    println!("Please pick a password for the user inside the Litterbox.");
    let password = Password::new("User password:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .prompt()
        .map_err(LitterboxError::PromptError)?;

    let image_name = gen_random_name();
    let child = Command::new("podman")
        .args([
            "build",
            "--build-arg",
            &format!("USER={}", user),
            "--build-arg",
            &format!("PASSWORD={}", password),
            "-t",
            &image_name,
            "--label",
            &format!("work.litterbox.name={lbx_name}"),
            "-f",
            dockerfile_path.to_str().expect("Invalid dockerfile_path."),
        ])
        .spawn()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    wait_for_podman(child)?;
    info!("Built image named {image_name}.");
    Ok(())
}

#[derive(Debug, Copy, Clone, Selectable)]
enum NetworkMode {
    Pasta,
    PastaWithForwarding,
    Host,
}

impl NetworkMode {
    fn name(&self) -> &'static str {
        match self {
            NetworkMode::Pasta => "Pasta (isolated user-mode networking stack)",
            NetworkMode::PastaWithForwarding => "Pasta with port forwarding (host to container)",
            NetworkMode::Host => "Host networking (i.e. NO ISOLATION)",
        }
    }

    fn podman_args(&self) -> &'static str {
        match self {
            NetworkMode::Pasta => "pasta",
            NetworkMode::PastaWithForwarding => "pasta:-t,auto,-u,auto",
            NetworkMode::Host => "host",
        }
    }
}

impl Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

pub fn build_litterbox(lbx_name: &str, user: &str) -> Result<(), LitterboxError> {
    match get_container_id(lbx_name) {
        Ok(id) => return Err(LitterboxError::ContainerAlreadyExists(id)),
        Err(LitterboxError::NoContainerForName) => {}
        Err(other) => return Err(other),
    };

    let image_id = get_image_id(lbx_name)?;
    let container_name = gen_random_name();

    let wayland_display = get_env("WAYLAND_DISPLAY")?;
    let xdg_runtime_dir = get_env("XDG_RUNTIME_DIR")?;

    let litterbox_home = lbx_home_path(lbx_name)?;
    fs::create_dir_all(&litterbox_home)
        .map_err(|e| LitterboxError::DirUncreatable(e, litterbox_home.clone()))?;

    let ssh_sock = SshSockFile::new(lbx_name, true)?;
    let ssh_sock_path = ssh_sock
        .path()
        .to_str()
        .expect("SSH socket path should be valid string");

    let network_mode = NetworkMode::select("Choose the network mode for this Litterbox:")
        .prompt()
        .map_err(LitterboxError::PromptError)?;

    let support_ping = Confirm::new("Do you want to support `ping` inside this Litterbox?")
        .with_default(false)
        .with_help_message("This will enable `CAP_NET_RAW`.")
        .prompt()
        .map_err(LitterboxError::PromptError)?;

    let support_tuntap =
        Confirm::new("Do you want to support TUN/TAP creation inside this Litterbox?")
            .with_default(false)
            .with_help_message("This will enable `CAP_NET_ADMIN` and expose `/dev/net/tun`.")
            .prompt()
            .map_err(LitterboxError::PromptError)?;

    let enable_packet_forwarding =
        Confirm::new("Do you want to enable packet forwarding inside this Litterbox?")
            .with_default(false)
            .prompt()
            .map_err(LitterboxError::PromptError)?;

    let base_args = &[
        "create",
        "--tty",
        "--name",
        &container_name,
        "--userns=keep-id",
        "--device",
        "/dev/dri",
        "--hostname",
        &format!("lbx-{lbx_name}"),
        "--network",
        network_mode.podman_args(),
        "--security-opt=label=disable", // TODO: use udica to make better rules instead
        "-e",
        "SSH_AUTH_SOCK=/tmp/ssh-agent.sock",
        "-v",
        &format!("{ssh_sock_path}:/tmp/ssh-agent.sock"),
        "-e",
        &format!("WAYLAND_DISPLAY={wayland_display}"),
        "-e",
        "XDG_RUNTIME_DIR=/tmp",
        "-v",
        &format!("{xdg_runtime_dir}/{wayland_display}:/tmp/{wayland_display}"),
        "-v",
        "/dev/dri:/dev/dri", // TODO: this does not work on WSL as the display device is different there
        "-v",
        &format!(
            "{}:/home/{user}",
            litterbox_home.to_str().expect("Invalid litterbox_home.")
        ),
        "--label",
        &format!("work.litterbox.name={lbx_name}"),
    ];
    let mut full_args = base_args.to_vec();

    if support_tuntap {
        debug!("Appending TUN/TAP args");
        full_args.extend_from_slice(&["--cap-add=NET_ADMIN", "--device", "/dev/net/tun"]);
    }

    if support_ping {
        debug!("Appending ping args");
        full_args.push("--cap-add=NET_RAW");
    }

    if enable_packet_forwarding {
        debug!("Appending packet forwarding args");
        full_args.extend_from_slice(&[
            "--sysctl",
            "net.ipv4.ip_forward=1",
            "--sysctl",
            "net.ipv6.conf.all.forwarding=1",
        ]);
    }

    // It's best to have the image_id as the final argument
    full_args.push(&image_id);

    debug!("build_litterbox full_args: {:#?}", full_args);

    let child = Command::new("podman")
        .args(full_args)
        .spawn()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    wait_for_podman(child)?;
    info!("Created container named {container_name}.");
    Ok(())
}

pub async fn enter_litterbox(lbx_name: &str) -> Result<(), LitterboxError> {
    let keys = crate::keys::Keys::load()?;
    keys.start_ssh_server(lbx_name).await?;

    let child = Command::new("podman")
        .args([
            "start",
            "--interactive",
            "--attach",
            &get_container_id(lbx_name)?,
        ])
        .spawn()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    wait_for_podman(child)?;
    debug!("Litterbox finished.");
    Ok(())
}

pub fn delete_litterbox(lbx_name: &str) -> Result<(), LitterboxError> {
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

    let child = Command::new("podman")
        .args(["rm", &container_id])
        .spawn()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    wait_for_podman(child)?;
    info!("Container for Litterbox deleted!");

    let image_id = get_image_id(lbx_name)?;
    let child = Command::new("podman")
        .args(["image", "rm", &image_id])
        .spawn()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    wait_for_podman(child)?;
    info!("Image for Litterbox deleted!");

    // TODO: ask the user if they also want the home dir deleted
    Ok(())
}
