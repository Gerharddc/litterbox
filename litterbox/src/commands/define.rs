use anyhow::{Result, bail};
use clap::Args;
use log::info;

use crate::{
    files::{dockerfile_path, write_file},
    template::Template,
};

/// Define a new Litterbox using a template Dockerfile
#[derive(Args, Debug)]
pub struct Command {
    /// The name of the Litterbox to define
    name: String,
}

impl Command {
    pub fn run(self) -> Result<()> {
        let dockerfile = dockerfile_path(&self.name)?;

        if dockerfile.exists() {
            bail!("Dockerfile already exists at {dockerfile:?}");
        }

        let template = Template::select("Choose a template:").prompt()?;

        write_file(dockerfile.as_path(), template.contents())?;
        info!("Default Dockerfile written to {dockerfile:?}");

        Ok(())
    }
}
