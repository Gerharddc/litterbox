use anyhow::Result;
use clap::Args;
use tabled::{Table, Tabled};

use crate::podman::{Container, get_containers};

#[derive(Tabled)]
struct ContainerTableRow {
    name: String,
    container_id: String,
    container_names: String,
    image: String,
    image_id: String,
}

impl From<&Container> for ContainerTableRow {
    fn from(value: &Container) -> Self {
        Self {
            name: value.labels.name.clone(),
            container_id: value.id.chars().take(12).collect(),
            container_names: value.names.join(","),
            image: value.image.clone(),
            image_id: value.image_id.chars().take(12).collect(),
        }
    }
}

/// List all the Litterboxes that have been created
#[derive(Args, Debug)]
pub struct Command {}

impl Command {
    pub fn run(self) -> Result<()> {
        let containers = get_containers()?;
        let table_rows: Vec<ContainerTableRow> = containers.0.iter().map(|c| c.into()).collect();
        let table = Table::new(table_rows);

        println!("{table}");

        Ok(())
    }
}
