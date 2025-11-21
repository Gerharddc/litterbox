use inquire::InquireError;
use log::error;
use std::{ffi::OsString, io, path::PathBuf, process::ExitStatus};

#[derive(Debug)]
pub enum LitterboxError {
    RunCommand(io::Error, &'static str),
    CommandFailed(ExitStatus, &'static str),
    PodmanError(ExitStatus, String),
    ParseOutput(std::str::Utf8Error),
    Deserialize(serde_json::error::Error),
    EnvVarUndefined(&'static str),
    EnvVarInvalid(&'static str, OsString),
    DirUncreatable(io::Error, PathBuf),
    WriteFailed(io::Error, PathBuf),
    ReadFailed(io::Error, PathBuf),
    ExistsFailed(io::Error, PathBuf),
    RemoveFailed(io::Error, PathBuf),
    NoContainerForName,
    MultipleContainersForName,
    ContainerAlreadyExists(String),
    NoImageForName,
    MultipleImagesForName,
    ImageAlreadyExists(String),
    DockerfileAlreadyExists(PathBuf),
    PromptError(InquireError),
    FailedToSerialise(&'static str),
    KeyAlreadyExists(String),
    KeyDoesNotExist(String),
    AlreadyAttachedToKey(String, String),
    Nix(nix::errno::Errno),
    InvalidDevicePath(String),
    ConnectSocket(io::Error),
    RegisterKey(russh::keys::Error),
    ParseKeyFile(ron::error::SpannedError),
}

impl LitterboxError {
    pub fn print(&self) {
        match self {
            LitterboxError::RunCommand(e, cmd) => {
                error!("{:#?}", e);
                eprintln!("Could not run {cmd} command. Perhaps it is not installed?");
            }
            LitterboxError::CommandFailed(exit_status, cmd) => {
                error!("error code: {:#?}", exit_status);
                eprintln!("{cmd} command failed with non-zero error code.");
            }
            LitterboxError::PodmanError(exit_status, stderr) => {
                error!("error code: {:#?}, message: {stderr}", exit_status);
                eprintln!("Podman command returned non-zero error code.");
            }
            LitterboxError::ParseOutput(e) => {
                error!("{:#?}", e);
                eprintln!("Could not parse output from podman.");
            }
            LitterboxError::Deserialize(e) => {
                error!("{:#?}", e);
                eprintln!("Could not deserialize output from podman. Unexpected format.");
            }
            LitterboxError::EnvVarUndefined(name) => {
                eprintln!("Environment variable not defined: {name}.")
            }
            LitterboxError::EnvVarInvalid(name, value) => {
                error!("{:#?}", value);
                eprintln!("Environment variable not a valid string: {name}.");
            }
            LitterboxError::DirUncreatable(error, dir) => {
                error!("{:#?}", error);
                eprintln!("Directory could not be created: {}.", dir.display());
            }
            LitterboxError::WriteFailed(error, path) => {
                error!("{:#?}", error);
                eprintln!("File could not be written: {}.", path.display());
            }
            LitterboxError::ReadFailed(error, path) => {
                error!("{:#?}", error);
                eprintln!("File could not be read: {}.", path.display());
            }
            LitterboxError::ExistsFailed(error, path) => {
                error!("{:#?}", error);
                eprintln!("Could not check if file exists: {}.", path.display());
            }
            LitterboxError::RemoveFailed(error, path) => {
                error!("{:#?}", error);
                eprintln!("Could not remove file: {}.", path.display());
            }
            LitterboxError::NoContainerForName => {
                eprintln!("A container with the specified Litterbox name could not be found.");
            }
            LitterboxError::MultipleContainersForName => {
                eprintln!("Multiple containers were found with the specified Litterbox name.");
            }
            LitterboxError::ContainerAlreadyExists(id) => {
                eprintln!("Container for Litterbox already exists with id: {id}.");
            }
            LitterboxError::NoImageForName => {
                eprintln!("An image with the specified Litterbox name could not be found.");
            }
            LitterboxError::MultipleImagesForName => {
                eprintln!("Multiple images were found with the specified Litterbox name.");
            }
            LitterboxError::ImageAlreadyExists(id) => {
                eprintln!("Image for Litterbox already exists with id: {id}.");
            }
            LitterboxError::DockerfileAlreadyExists(path) => {
                eprintln!(
                    "Dockerfile for Litterbox already exists at {}.",
                    path.display()
                );
            }
            LitterboxError::PromptError(error) => {
                error!("{:#?}", error);
                eprintln!("Failed to retrieve valid input from user.");
            }
            LitterboxError::FailedToSerialise(name) => {
                eprintln!("Failed to serialise {name}.");
            }
            LitterboxError::KeyAlreadyExists(name) => {
                eprintln!("Key named {name} already exists.")
            }
            LitterboxError::KeyDoesNotExist(name) => {
                eprintln!("Key named {name} does not exist.")
            }
            LitterboxError::AlreadyAttachedToKey(key_name, litterbox_name) => {
                eprintln!(
                    "Litterbox named {litterbox_name} already attached to key named {key_name}."
                )
            }
            LitterboxError::Nix(errno) => {
                eprintln!("Linux error: {:#?}", errno);
            }
            LitterboxError::InvalidDevicePath(path) => {
                eprintln!("The following device path is not valid: {:#?}", path);
            }
            LitterboxError::ConnectSocket(error) => {
                error!("{:#?}", error);
                eprintln!("Failed to connect to socket.");
            }
            LitterboxError::RegisterKey(error) => {
                error!("{:#?}", error);
                eprintln!("Failed to register SSH key with internal agent.");
            }
            LitterboxError::ParseKeyFile(error) => {
                error!("{:#?}", error);
                eprintln!("Failed to parse keyfile.");
            }
        }
    }
}
