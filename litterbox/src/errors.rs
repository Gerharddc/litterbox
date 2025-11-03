use inquire::InquireError;
use log::error;
use std::{ffi::OsString, io, path::PathBuf, process::ExitStatus};

#[derive(Debug)]
pub enum LitterboxError {
    RunPodman(io::Error),
    PodmanError(ExitStatus, String),
    ParseOutput(std::str::Utf8Error),
    Deserialize(serde_json::error::Error),
    EnvVarUndefined(&'static str),
    EnvVarInvalid(&'static str, OsString),
    DirUncreatable(io::Error, PathBuf),
    WriteFailed(io::Error, PathBuf),
    ReadFailed(io::Error, PathBuf),
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
}

impl LitterboxError {
    pub fn print(&self) {
        match self {
            LitterboxError::RunPodman(e) => {
                error!("{:#?}", e);
                println!("Could not run podman command. Perhaps it is not installed?");
            }
            LitterboxError::PodmanError(exit_status, stderr) => {
                error!("error code: {:#?}, message: {stderr}", exit_status);
                println!("Podman command returned non-zero error code.");
            }
            LitterboxError::ParseOutput(e) => {
                error!("{:#?}", e);
                println!("Could not parse output from podman.");
            }
            LitterboxError::Deserialize(e) => {
                error!("{:#?}", e);
                println!("Could not deserialize output from podman. Unexpected format.");
            }
            LitterboxError::EnvVarUndefined(name) => {
                println!("Environment variable not defined: {name}.")
            }
            LitterboxError::EnvVarInvalid(name, value) => {
                error!("{:#?}", value);
                println!("Environment variable not a valid string: {name}.");
            }
            LitterboxError::DirUncreatable(error, dir) => {
                error!("{:#?}", error);
                println!("Directory could not be created: {}.", dir.display());
            }
            LitterboxError::WriteFailed(error, path) => {
                error!("{:#?}", error);
                println!("File could not be written: {}.", path.display());
            }
            LitterboxError::ReadFailed(error, path) => {
                error!("{:#?}", error);
                println!("File could not be read: {}.", path.display());
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
                println!(
                    "Dockerfile for Litterbox already exists at {}.",
                    path.display()
                );
            }
            LitterboxError::PromptError(error) => {
                error!("{:#?}", error);
                println!("Failed to retrieve valid input from user.");
            }
            LitterboxError::FailedToSerialise(name) => {
                println!("Failed to serialise {name}.");
            }
            LitterboxError::KeyAlreadyExists(name) => {
                println!("Key named {name} already exists.")
            }
            LitterboxError::KeyDoesNotExist(name) => {
                println!("Key named {name} does not exist.")
            }
            LitterboxError::AlreadyAttachedToKey(key_name, litterbox_name) => {
                println!(
                    "Litterbox named {litterbox_name} already attached to key named {key_name}."
                )
            }
            LitterboxError::Nix(errno) => {
                println!("Linux error: {:#?}", errno);
            }
            LitterboxError::InvalidDevicePath(path) => {
                println!("The following device path is not valid: {:#?}", path);
            }
        }
    }
}
