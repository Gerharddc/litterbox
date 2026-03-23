use anyhow::{Result, bail};
use clap::Parser;
use std::process::Output;

mod agent;
mod commands;
mod daemon;
mod devices;
mod env;
mod files;
mod keys;
mod podman;
mod sandbox;
mod settings;
mod template;
mod utils;

use crate::keys::Keys;

pub fn extract_stdout(output: &Output) -> Result<&str> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        bail!("Command failed: {stderr}");
    }

    Ok(str::from_utf8(&output.stdout)?)
}

pub fn generate_name() -> String {
    let mut generator = names::Generator::with_naming(names::Name::Numbered);
    let name = generator.next().expect("Name should not be None");

    format!("lbx-{name}")
}

/// Simple sandbox utility aimed at software development
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: crate::commands::Command,
}

fn main() -> Result<()> {
    env_logger::init();

    let argv_0 = std::env::args().next();
    if matches!(argv_0.as_deref(), Some("run0" | "sudo")) {
        eprintln!(
            "run0/sudo is not supported inside this session. Use 'litterbox enter --root <name>' to enter as root."
        );

        return Ok(());
    }

    let args = Args::parse();

    args.command.run()
}
