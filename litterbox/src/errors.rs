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
    CreateFailed(io::Error, PathBuf),
    NoContainerForName,
    MultipleContainersForName,
    NoImageForName,
    MultipleImagesForName,
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
    ParseSettingsFile(ron::error::SpannedError),
    ReplaceNotAllowed,
    InvalidInput(String),
}

impl LitterboxError {
    pub fn print(&self) {
        match self {
            LitterboxError::RunCommand(e, cmd) => {
                error!("{:#?}", e);
                error!("Could not run {cmd} command. Perhaps it is not installed?");
            }
            LitterboxError::CommandFailed(exit_status, cmd) => {
                error!("error code: {:#?}", exit_status);
                error!("{cmd} command failed with non-zero error code.");
            }
            LitterboxError::PodmanError(exit_status, stderr) => {
                error!("error code: {:#?}, message: {stderr}", exit_status);
                error!("Podman command returned non-zero error code.");
            }
            LitterboxError::ParseOutput(e) => {
                error!("{:#?}", e);
                error!("Could not parse output from podman.");
            }
            LitterboxError::Deserialize(e) => {
                error!("{:#?}", e);
                error!("Could not deserialize output from podman. Unexpected format.");
            }
            LitterboxError::EnvVarUndefined(name) => {
                error!("Environment variable not defined: {name}.")
            }
            LitterboxError::EnvVarInvalid(name, value) => {
                error!("{:#?}", value);
                error!("Environment variable not a valid string: {name}.");
            }
            LitterboxError::DirUncreatable(error, dir) => {
                error!("{:#?}", error);
                error!("Directory could not be created: {}.", dir.display());
            }
            LitterboxError::WriteFailed(error, path) => {
                error!("{:#?}", error);
                error!("File could not be written: {}.", path.display());
            }
            LitterboxError::ReadFailed(error, path) => {
                error!("{:#?}", error);
                error!("File could not be read: {}.", path.display());
            }
            LitterboxError::ExistsFailed(error, path) => {
                error!("{:#?}", error);
                error!("Could not check if file exists: {}.", path.display());
            }
            LitterboxError::RemoveFailed(error, path) => {
                error!("{:#?}", error);
                error!("Could not remove file: {}.", path.display());
            }
            LitterboxError::CreateFailed(error, path) => {
                error!("{:#?}", error);
                error!("Could not create file: {}.", path.display());
            }
            LitterboxError::NoContainerForName => {
                error!("A container with the specified Litterbox name could not be found.");
            }
            LitterboxError::MultipleContainersForName => {
                error!("Multiple containers were found with the specified Litterbox name.");
            }
            LitterboxError::NoImageForName => {
                error!("An image with the specified Litterbox name could not be found.");
            }
            LitterboxError::MultipleImagesForName => {
                error!("Multiple images were found with the specified Litterbox name.");
            }
            LitterboxError::DockerfileAlreadyExists(path) => {
                error!(
                    "Dockerfile for Litterbox already exists at {}.",
                    path.display()
                );
            }
            LitterboxError::PromptError(error) => {
                error!("{:#?}", error);
                error!("Failed to retrieve valid input from user.");
            }
            LitterboxError::FailedToSerialise(name) => {
                error!("Failed to serialise {name}.");
            }
            LitterboxError::KeyAlreadyExists(name) => {
                error!("Key named {name} already exists.")
            }
            LitterboxError::KeyDoesNotExist(name) => {
                error!("Key named {name} does not exist.")
            }
            LitterboxError::AlreadyAttachedToKey(key_name, litterbox_name) => {
                error!("Litterbox named {litterbox_name} already attached to key named {key_name}.")
            }
            LitterboxError::Nix(errno) => {
                error!("Linux error: {:#?}", errno);
            }
            LitterboxError::InvalidDevicePath(path) => {
                error!("The following device path is not valid: {:#?}", path);
            }
            LitterboxError::ConnectSocket(error) => {
                error!("{:#?}", error);
                error!("Failed to connect to socket.");
            }
            LitterboxError::RegisterKey(error) => {
                error!("{:#?}", error);
                error!("Failed to register SSH key with internal agent.");
            }
            LitterboxError::ParseKeyFile(error) => {
                error!("{:#?}", error);
                error!("Failed to parse keyfile.");
            }
            LitterboxError::ParseSettingsFile(error) => {
                error!("{:#?}", error);
                error!("Failed to parse settings file.");
            }
            LitterboxError::ReplaceNotAllowed => {
                error!("Litterbox cannot be rebuilt without replacing container.");
            }
            LitterboxError::InvalidInput(msg) => {
                error!("Invalid input: {msg}");
            }
        }
    }
}
