use inquire::{Confirm, Password};
use log::{debug, info, warn};
use serde::Deserialize;
use std::{
    fs,
    path::Path,
    process::{Child, Command},
};

use crate::{
    define_litterbox, env,
    errors::LitterboxError,
    extract_stdout,
    files::{SshSockFile, dockerfile_path, lbx_home_path, pipewire_socket_path},
    gen_random_name,
    settings::LitterboxSettings,
};

/// Represents the GPU device configuration for the container
enum GpuDevice {
    /// Standard Linux GPU device at /dev/dri
    Dri,
    /// WSL2 DirectX device at /dev/dxg
    Dxg,
}

impl GpuDevice {
    fn device_path(&self) -> &'static str {
        match self {
            GpuDevice::Dri => "/dev/dri",
            GpuDevice::Dxg => "/dev/dxg",
        }
    }

    fn volume_mount(&self) -> &'static str {
        match self {
            GpuDevice::Dri => "/dev/dri:/dev/dri",
            GpuDevice::Dxg => "/dev/dxg:/dev/dxg",
        }
    }
}

/// Detects the available GPU device based on what exists on the system
fn detect_gpu_device() -> Option<GpuDevice> {
    if Path::new("/dev/dri").exists() {
        debug!("/dev/dri available");
        Some(GpuDevice::Dri)
    } else if Path::new("/dev/dxg").exists() {
        debug!("/dev/dxg available (WSL)");
        Some(GpuDevice::Dxg)
    } else {
        None
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct LitterboxLabels {
    #[serde(rename = "work.litterbox.name")]
    pub name: String,
}

#[derive(Deserialize, Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
pub struct ImageDetails {
    #[serde(rename = "Id")]
    pub id: String,

    #[serde(rename = "Names")]
    pub names: Vec<String>,
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

pub fn get_container_details(lbx_name: &str) -> Result<ContainerDetails, LitterboxError> {
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
        1 => Ok(containers.0[0].clone()),
        _ => Err(LitterboxError::MultipleContainersForName),
    }
}

pub fn get_image_details(lbx_name: &str) -> Result<ImageDetails, LitterboxError> {
    let output = Command::new("podman")
        .args([
            "image",
            "ls",
            "-a",
            "--format",
            "json",
            "--filter",
            &format!("label=work.litterbox.name={lbx_name}"),
            "--filter", // We need to avoid dangling images that are left behind when an image gets rebuilt
            "dangling=false",
        ])
        .output()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    let stdout = extract_stdout(&output)?;
    let images: AllImages = serde_json::from_str(stdout).map_err(LitterboxError::Deserialize)?;

    match images.0.len() {
        0 => Err(LitterboxError::NoImageForName),
        1 => Ok(images.0[0].clone()),
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
    let image_name = match get_image_details(lbx_name) {
        Ok(details) => {
            assert!(!details.names.is_empty(), "All images should have a name.");
            if details.names.len() > 1 {
                warn!("Image for Litterbox had more than one name. The first one will be used.");
            }

            println!("An image for this Litterbox already exists.");
            if Confirm::new("Would you like to rebuild the image?")
                .with_default(true)
                .prompt()
                .map_err(LitterboxError::PromptError)?
            {
                println!("The image will now be rebuilt!");
            } else {
                println!("The existing image will be re-used!");

                // Exit the whole function since we don't need to do anything more
                return Ok(());
            }
            Ok(details.names[0].clone())
        }
        Err(LitterboxError::NoImageForName) => Ok(gen_random_name()),
        Err(other) => Err(other),
    }?;

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

pub fn build_litterbox(lbx_name: &str, user: &str) -> Result<(), LitterboxError> {
    let image_id = get_image_details(lbx_name)?.id;
    let container_name = match get_container_details(lbx_name) {
        Ok(details) => {
            assert!(
                !details.names.is_empty(),
                "All containers should have a name."
            );
            if details.names.len() > 1 {
                warn!(
                    "Container for Litterbox had more than one name. The first one will be used."
                );
            }

            println!("A container for this Litterbox already exists.");
            if Confirm::new("Would you like to replace this container?")
                .with_default(true)
                .prompt()
                .map_err(LitterboxError::PromptError)?
            {
                println!("The container will now be replaced!");
                Ok(details.names[0].clone())
            } else {
                // It is meaningless to "build" the Litterbox if we are not allowed to replace the container.
                // Hence, we throw and error here.
                Err(LitterboxError::ReplaceNotAllowed)
            }
        }
        Err(LitterboxError::NoContainerForName) => Ok(gen_random_name()),
        Err(other) => Err(other),
    }?;

    let wayland_display = env::wayland_display()?;
    let xdg_runtime_dir = env::xdg_runtime_dir()?;

    let litterbox_home = lbx_home_path(lbx_name)?;
    fs::create_dir_all(&litterbox_home)
        .map_err(|e| LitterboxError::DirUncreatable(e, litterbox_home.clone()))?;

    let ssh_sock = SshSockFile::new(lbx_name, true)?;
    let ssh_sock_path = ssh_sock
        .path()
        .to_str()
        .expect("SSH socket path should be valid string");
    let ssh_sock_mount = format!("{ssh_sock_path}:/tmp/ssh-agent.sock");

    let settings = LitterboxSettings::load_or_prompt(lbx_name)?;

    let gpu_device = detect_gpu_device();
    if gpu_device.is_none() {
        warn!("No GPU device found. GPU acceleration will not be available in the Litterbox.");
    }

    let hostname = format!("lbx-{lbx_name}");
    let wayland_display_env = format!("WAYLAND_DISPLAY={wayland_display}");
    let wayland_socket_mount =
        format!("{xdg_runtime_dir}/{wayland_display}:/tmp/{wayland_display}");
    let home_mount = format!(
        "{}:/home/{user}",
        litterbox_home.to_str().expect("Invalid litterbox_home.")
    );
    let label = format!("work.litterbox.name={lbx_name}");

    let mut full_args = vec![
        "create",
        "--tty",
        "--replace",
        "--name",
        &container_name,
        "--userns=keep-id",
        "--hostname",
        &hostname,
        "--network",
        settings.network_mode.podman_args(),
        "--security-opt=label=disable", // TODO: use Landlock for better isolation
        "-e",
        "SSH_AUTH_SOCK=/tmp/ssh-agent.sock",
        "-v",
        &ssh_sock_mount,
        "-e",
        &wayland_display_env,
        "-e",
        "XDG_SESSION_TYPE=wayland",
        "-e",
        "XDG_RUNTIME_DIR=/tmp",
        "-v",
        &wayland_socket_mount,
        "-v",
        &home_mount,
        "--label",
        &label,
    ];

    if let Some(gpu) = &gpu_device {
        debug!("Appending GPU device args for {}", gpu.device_path());
        full_args.extend_from_slice(&["--device", gpu.device_path(), "-v", gpu.volume_mount()]);
    }

    if settings.support_tuntap {
        debug!("Appending TUN/TAP args");
        full_args.extend_from_slice(&["--cap-add=NET_ADMIN", "--device", "/dev/net/tun"]);
    }

    if settings.support_ping {
        debug!("Appending ping args");
        full_args.push("--cap-add=NET_RAW");
    }

    if settings.packet_forwarding {
        debug!("Appending packet forwarding args");
        full_args.extend_from_slice(&[
            "--sysctl",
            "net.ipv4.ip_forward=1",
            "--sysctl",
            "net.ipv6.conf.all.forwarding=1",
        ]);
    }

    if settings.enable_kvm {
        debug!("Appending KVM device args");
        full_args.extend_from_slice(&["--device", "/dev/kvm"]);
    }

    let pipewire_path = pipewire_socket_path()?;
    let pipewire_path = pipewire_path.to_str().expect("Path should be valid string");
    let pipewire_socket_mount = format!("{pipewire_path}:/tmp/pipewire-0");
    if settings.expose_pipewire {
        debug!("Appending PipeWire socket args");
        full_args.extend_from_slice(&["-v", &pipewire_socket_mount]);
    }

    if settings.keep_groups {
        debug!("Appending keep groups args");
        full_args.push("--group-add=keep-groups");
    }

    if settings.expose_kfd {
        debug!("Appending KFD device args");
        full_args.extend_from_slice(&["--device", "/dev/kfd"]);
    }

    let shm_size_arg = settings.shm_size_gb.map(|gb| format!("{}G", gb));
    if let Some(ref shm_size) = shm_size_arg {
        debug!("Appending shm-size args: {}", shm_size);
        full_args.extend_from_slice(&["--shm-size", shm_size]);
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
            &get_container_details(lbx_name)?.id,
        ])
        .spawn()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    wait_for_podman(child)?;
    debug!("Litterbox finished.");
    Ok(())
}

pub fn delete_litterbox(lbx_name: &str) -> Result<(), LitterboxError> {
    // We check if it exists before promting the user
    let container_id = get_container_details(lbx_name)?.id;

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

    let image_details = get_image_details(lbx_name)?;
    let child = Command::new("podman")
        .args(["image", "rm", &image_details.id])
        .spawn()
        .map_err(|e| LitterboxError::RunCommand(e, "podman"))?;

    wait_for_podman(child)?;
    info!("Image for Litterbox deleted!");

    // TODO: ask the user if they also want the home dir deleted
    Ok(())
}
