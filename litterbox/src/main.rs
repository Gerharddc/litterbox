use std::env::VarError;

use clap::Parser;

mod agent;
mod commands;
mod daemon;
mod devices;
mod entrypoint;
mod env;
mod files;
mod keys;
mod podman;
mod sandbox;
mod settings;
mod template;
mod utils;

use crate::{keys::Keys, utils::SU_BINARIES};

/// Simple sandbox utility aimed at software development
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: crate::commands::Command,
}

fn main() -> anyhow::Result<()> {
    let arg0 = std::env::args().next();

    if let Some(arg0) = arg0.as_deref()
        && SU_BINARIES.contains(&arg0)
    {
        eprintln!(
            "{arg0:?} is not supported inside this session. Use 'litterbox enter --root NAME' to enter as root."
        );

        std::process::exit(1);
    }

    let args = Args::parse();

    // Configure default log level for debug and release builds.
    if std::env::var("RUST_LOG").is_err_and(|e| e == VarError::NotPresent) {
        // SAFETY: No other threads are reading or writing to env variables.
        unsafe {
            #[cfg(debug_assertions)]
            std::env::set_var("RUST_LOG", "debug");

            #[cfg(not(debug_assertions))]
            std::env::set_var("RUST_LOG", "info");
        }
    }

    env_logger::init();
    args.command.run()
}
