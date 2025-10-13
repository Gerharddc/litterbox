use inquire::InquireError;
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
                let dir = dir.display();
                println!("Directory could not be created: {dir}.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
            }
            LitterboxError::WriteFailed(error, path) => {
                let path = path.display();
                println!("File could not be written: {path}.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
            }
            LitterboxError::ReadFailed(error, path) => {
                let path = path.display();
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
                let path = path.display();
                println!("Dockerfile for Litterbox already exists at {path}.");
            }
            LitterboxError::PromptError(error) => {
                println!("Failed to retrieve valid input from user.");

                // TODO: use env_logger instead
                eprintln!("{:#?}", error);
            }
            LitterboxError::FailedToSerialise(name) => {
                println!("Failed to serialise {name}");
            }
        }
    }
}
