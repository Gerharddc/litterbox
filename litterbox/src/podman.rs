use anyhow::{Context, Result, anyhow, ensure};
use inquire::{Confirm, Password};
use log::{debug, info, warn};
use serde::Deserialize;
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use crate::{
    daemon, define_litterbox, env, extract_stdout,
    files::{self, SshSockFile},
    gen_random_name,
    keys::Keys,
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

pub fn list_containers() -> Result<AllContainers> {
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
        .context("Failed to run podman command")?;

    let stdout = extract_stdout(&output)?;
    Ok(serde_json::from_str(stdout)?)
}

pub fn get_container_details(lbx_name: &str) -> Result<Option<ContainerDetails>> {
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
        .context("Failed to run podman command")?;

    let stdout = extract_stdout(&output)?;
    let containers: AllContainers = serde_json::from_str(stdout)?;

    match containers.0.len() {
        0 => Ok(None),
        1 => Ok(Some(containers.0[0].clone())),
        _ => Err(anyhow!("Multiple containers found for {}", lbx_name)),
    }
}

pub fn get_image_details(lbx_name: &str) -> Result<Option<ImageDetails>> {
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
        .context("Failed to run podman command")?;

    let stdout = extract_stdout(&output)?;
    let images: AllImages = serde_json::from_str(stdout)?;

    match images.0.len() {
        0 => Ok(None),
        1 => Ok(Some(images.0[0].clone())),
        _ => Err(anyhow!("Multiple images found for {}", lbx_name)),
    }
}

pub fn is_container_running(lbx_name: &str) -> Result<bool> {
    let output = Command::new("podman")
        .args([
            "ps",
            "--format",
            "json",
            "--filter",
            &format!("label=work.litterbox.name={lbx_name}"),
        ])
        .output()
        .context("Failed to run podman command")?;

    let stdout = extract_stdout(&output)?;
    let containers: AllContainers = serde_json::from_str(stdout)?;

    Ok(!containers.0.is_empty())
}

fn wait_for_podman(mut child: Child) -> Result<()> {
    let res = child.wait().context("Failed to run podman command")?;
    ensure!(res.success(), "Podman command failed");
    Ok(())
}

async fn wait_for_podman_async(child: &mut tokio::process::Child) -> Result<()> {
    let res = child.wait().await.context("Failed to run podman command")?;
    ensure!(res.success(), "Podman command failed");
    Ok(())
}

pub fn build_image(lbx_name: &str, user: &str) -> Result<()> {
    let image_name = match get_image_details(lbx_name)? {
        Some(details) => {
            assert!(!details.names.is_empty(), "All images should have a name.");
            if details.names.len() > 1 {
                warn!("Image for Litterbox had more than one name. The first one will be used.");
            }

            println!("An image for this Litterbox already exists.");
            if Confirm::new("Would you like to rebuild the image?")
                .with_default(true)
                .prompt()?
            {
                println!("The image will now be rebuilt!");
            } else {
                println!("The existing image will be re-used!");

                // Exit the whole function since we don't need to do anything more
                return Ok(());
            }
            details.names[0].clone()
        }
        None => gen_random_name(),
    };

    let dockerfile_path = files::dockerfile_path(lbx_name)?;
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
        .prompt()?;

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
        .context("Failed to run podman command")?;

    wait_for_podman(child)?;
    info!("Built image named {image_name}.");
    Ok(())
}

pub fn build_litterbox(lbx_name: &str, user: &str) -> Result<()> {
    let image_details = get_image_details(lbx_name)?.ok_or_else(|| {
        anyhow!(
            "No image found for {}. Run `litterbox build` first.",
            lbx_name
        )
    })?;
    let image_id = image_details.id;
    let container_name = match get_container_details(lbx_name)? {
        Some(details) => {
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
                .prompt()?
            {
                println!("The container will now be replaced!");
                details.names[0].clone()
            } else {
                return Err(anyhow!("Cannot build without replacing container"));
            }
        }
        None => gen_random_name(),
    };

    let wayland_display = env::wayland_display()?;
    let xdg_runtime_dir = env::xdg_runtime_dir()?;

    let litterbox_home = files::lbx_home_path(lbx_name)?;
    fs::create_dir_all(&litterbox_home).context("Failed to create litterbox home directory")?;

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

    let litterbox_mount = format!("{}:/litterbox:ro", env::litterbox_binary_path());

    let session_lock_path = files::session_lock_path(lbx_name)?;
    let session_lock_path_str = session_lock_path
        .to_str()
        .expect("Session lock path should be valid.");

    if session_lock_path.exists() {
        log::warn!("Deleting old session lock file: {:#?}", session_lock_path);
        fs::remove_file(&session_lock_path)?;
    } else if let Some(parent) = session_lock_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::File::create(&session_lock_path)?;

    let session_lock_mount = format!("{session_lock_path_str}:/session.lock:ro");

    let mut full_args = vec![
        "create",
        "--replace",
        "--name",
        &container_name,
        "--userns=keep-id",
        "--hostname",
        &hostname,
        "--network",
        settings.network_mode.podman_args(),
        "--security-opt=label=disable", // TODO: use Landlock for better isolation
        "-v",
        &litterbox_mount,
        "-v",
        &session_lock_mount,
        "--entrypoint",
        "[\"/litterbox\", \"wait\"]",
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

    let pipewire_path = files::pipewire_socket_path()?;
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

    if settings.unconfine_seccomp {
        debug!("Disabling seccomp confinement");
        full_args.extend_from_slice(&["--security-opt", "seccomp=unconfined"]);
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
        .context("Failed to run podman command")?;

    wait_for_podman(child)?;
    info!("Created container named {container_name}.");
    Ok(())
}

pub fn enter_litterbox(
    lbx_name: &str,
    interactive: bool,
    tty: bool,
    workdir: Option<PathBuf>,
    command: Option<OsString>,
    command_args: Vec<OsString>,
) -> Result<()> {
    let container = get_container_details(lbx_name)?
        .ok_or_else(|| anyhow!("No container found for {}", lbx_name))?;
    let container_id = container.id;

    if !daemon::is_running(lbx_name)? {
        if is_container_running(lbx_name)? {
            warn!("Daemon was not running but container was. Restarting daemon...");
        }

        let keys = Keys::load()?;
        let password = keys.password_if_needed(lbx_name)?;

        let log_file = files::daemon_log_file(lbx_name)?;
        let log_file_clone = log_file.try_clone()?;

        let mut cmd = Command::new(env::litterbox_binary_path());
        cmd.args(["daemon", lbx_name]);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::from(log_file));
        cmd.stderr(Stdio::from(log_file_clone));

        let mut daemon_child = cmd.spawn().context("Failed to run Litterbox daemon")?;

        if let Some(ref pwd) = password
            && let Some(stdin) = daemon_child.stdin.take()
        {
            use std::io::Write;
            let mut stdin = stdin;
            stdin
                .write_all(pwd.as_bytes())
                .context("Failed to write password to daemon")?;
        }
    }

    let my_pid = std::process::id();
    let session_lock = files::session_lock_path(lbx_name)?;
    files::append_pid_to_session_lockfile(&session_lock, my_pid)?;

    if !is_container_running(lbx_name)? {
        println!("Container not running yet, starting now...");

        let start_child = Command::new("podman")
            .args(["start", &container_id])
            .spawn()
            .context("Failed to run podman command")?;

        wait_for_podman(start_child)?;
    } else {
        println!("Container already running, just attaching...")
    }

    tokio::runtime::Runtime::new()
        .expect("Tokio runtime should start")
        .block_on(async move {
            use tokio::process::Command;

            let mut exec_child = Command::new("podman");

            exec_child.arg("exec");

            // Assume -t if we are launching the login shell
            if tty || command.is_none() {
                exec_child.arg("--tty");
            }

            // Assume -i if we are launching the login shell
            if interactive || command.is_none() {
                exec_child.arg("--interactive");
            }

            if let Some(workdir) = workdir {
                exec_child.arg("--workdir");
                exec_child.arg(workdir.into_os_string());
            }

            exec_child.args([&container_id, "/litterbox", "setup-home"]);

            if let Some(command) = command {
                exec_child.arg(command);
                exec_child.args(command_args);
            }

            let mut exec_child = exec_child.spawn().context("Failed to run podman command")?;

            println!("Waiting for podman");
            tokio::select! {
                _ = wait_for_podman_async(&mut exec_child) => {}
                _ = tokio::signal::ctrl_c() => {
                    let _ = exec_child.kill().await;
                }
            }

            Result::<()>::Ok(())
        })?;

    files::remove_pid_from_session_lockfile(&session_lock, my_pid)?;
    debug!("Litterbox finished.");
    Ok(())
}

pub fn delete_litterbox(lbx_name: &str) -> Result<()> {
    let container = get_container_details(lbx_name)?
        .ok_or_else(|| anyhow!("No container found for {}", lbx_name))?;
    let container_id = container.id;

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
        .context("Failed to run podman command")?;

    wait_for_podman(child)?;
    info!("Container for Litterbox deleted!");

    let image_details =
        get_image_details(lbx_name)?.ok_or_else(|| anyhow!("No image found for {}", lbx_name))?;
    let child = Command::new("podman")
        .args(["image", "rm", &image_details.id])
        .spawn()
        .context("Failed to run podman command")?;

    wait_for_podman(child)?;
    info!("Image for Litterbox deleted!");

    let home_path = files::lbx_home_path(lbx_name)?;
    if home_path.exists() {
        let should_delete_home =
            Confirm::new("Do you want to delete the home directory for this Litterbox?")
                .with_default(false)
                .with_help_message(&format!("This will delete {}", home_path.display()))
                .prompt();

        match should_delete_home {
            Ok(true) => {
                fs::remove_dir_all(&home_path)?;
                info!("Home directory deleted!");
            }
            _ => {
                println!("Skipping home directory deletion.");
            }
        }
    }

    let dockerfile_path = files::dockerfile_path(lbx_name)?;
    let settings_path = files::settings_path(lbx_name)?;
    if dockerfile_path.exists() || settings_path.exists() {
        let should_delete_definition =
            Confirm::new("Do you want to delete the definition files for this Litterbox?")
                .with_default(false)
                .with_help_message("This will delete the Dockerfile and settings file")
                .prompt();

        match should_delete_definition {
            Ok(true) => {
                if dockerfile_path.exists() {
                    fs::remove_file(&dockerfile_path)?;
                    info!("Dockerfile deleted!")
                }

                if settings_path.exists() {
                    fs::remove_file(&settings_path)?;
                    info!("Settings file deleted!")
                }
            }
            _ => {
                println!("Skipping definition file deletion.");
            }
        }
    }

    Ok(())
}
