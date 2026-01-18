use inquire::Confirm;
use inquire_derive::Selectable;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::{
    errors::LitterboxError,
    files::{pipewire_socket_path, read_file, settings_path, write_file},
};

#[derive(Debug, Copy, Clone, Selectable, Serialize, Deserialize, PartialEq)]
pub enum NetworkMode {
    Pasta,
    PastaWithForwarding,
    Host,
}

impl NetworkMode {
    fn name(&self) -> &'static str {
        match self {
            NetworkMode::Pasta => "Pasta (isolated user-mode networking stack)",
            NetworkMode::PastaWithForwarding => "Pasta with port forwarding (host to container)",
            NetworkMode::Host => "Host networking (i.e. NO ISOLATION)",
        }
    }

    pub fn podman_args(&self) -> &'static str {
        match self {
            NetworkMode::Pasta => "pasta",
            NetworkMode::PastaWithForwarding => "pasta:-t,auto,-u,auto",
            NetworkMode::Host => "host",
        }
    }
}

impl Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Settings for a Litterbox container, persisted to disk as RON.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LitterboxSettings {
    /// Version of the settings format stored for future migrations
    pub version: u32,

    pub network_mode: NetworkMode,
    pub support_ping: bool,
    pub support_tuntap: bool,
    pub packet_forwarding: bool,
    pub enable_kvm: bool,
    pub expose_pipewire: bool,
}

impl LitterboxSettings {
    /// Load existing settings if available, prompt user if they want to change them,
    /// and save the final settings. This is the main entry point for getting settings
    /// during a build.
    pub fn load_or_prompt(lbx_name: &str) -> Result<Self, LitterboxError> {
        let existing = Self::load(lbx_name)?;

        let settings = match &existing {
            Some(existing) => {
                if Confirm::new("Would you like to change the settings for this Litterbox?")
                    .with_default(false)
                    .prompt()
                    .map_err(LitterboxError::PromptError)?
                {
                    Self::prompt(Some(existing))?
                } else {
                    existing.clone()
                }
            }
            None => Self::prompt(None)?,
        };

        settings.save_to_file(lbx_name)?;
        Ok(settings)
    }

    fn load(lbx_name: &str) -> Result<Option<Self>, LitterboxError> {
        let path = settings_path(lbx_name)?;
        if !path.exists() {
            debug!("Settings file does not exist for {}", lbx_name);
            return Ok(None);
        }

        let contents = read_file(&path)?;
        let settings: Self = ron::from_str(&contents).map_err(LitterboxError::ParseSettingsFile)?;
        Ok(Some(settings))
    }

    fn save_to_file(&self, lbx_name: &str) -> Result<(), LitterboxError> {
        use ron::ser::{PrettyConfig, to_string_pretty};

        let path = settings_path(lbx_name)?;
        let contents = to_string_pretty(self, PrettyConfig::default()).map_err(|e| {
            eprintln!("Serialise error: {:#?}", e);
            LitterboxError::FailedToSerialise("LitterboxSettings")
        })?;
        write_file(&path, &contents)
    }

    fn prompt(existing: Option<&Self>) -> Result<Self, LitterboxError> {
        let network_mode = NetworkMode::select("Choose the network mode for this Litterbox:")
            .with_starting_cursor(existing.map(|s| s.network_mode as usize).unwrap_or(0))
            .prompt()
            .map_err(LitterboxError::PromptError)?;

        let support_ping = Confirm::new("Do you want to support `ping` inside this Litterbox?")
            .with_default(existing.map(|s| s.support_ping).unwrap_or(false))
            .with_help_message("This will enable `CAP_NET_RAW`.")
            .prompt()
            .map_err(LitterboxError::PromptError)?;

        let support_tuntap =
            Confirm::new("Do you want to support TUN/TAP creation inside this Litterbox?")
                .with_default(existing.map(|s| s.support_tuntap).unwrap_or(false))
                .with_help_message("This will enable `CAP_NET_ADMIN` and expose `/dev/net/tun`.")
                .prompt()
                .map_err(LitterboxError::PromptError)?;

        let packet_forwarding =
            Confirm::new("Do you want to enable packet forwarding inside this Litterbox?")
                .with_default(existing.map(|s| s.packet_forwarding).unwrap_or(false))
                .prompt()
                .map_err(LitterboxError::PromptError)?;

        let enable_kvm = Confirm::new("Do you want to enable KVM support in this Litterbox?")
            .with_default(existing.map(|s| s.enable_kvm).unwrap_or(false))
            .with_help_message("This will expose '/dev/kvm' to the Litterbox.")
            .prompt()
            .map_err(LitterboxError::PromptError)?;

        let expose_pipewire = if pipewire_socket_path()?.exists() {
            Confirm::new("Do you want to expose PipeWire inside this Litterbox?")
                .with_default(existing.map(|s| s.expose_pipewire).unwrap_or(false))
                .with_help_message(
                    "This will allow audio applications to work inside the Litterbox.",
                )
                .prompt()
                .map_err(LitterboxError::PromptError)?
        } else {
            debug!("PipeWire socket not found on host system, user not prompted to expose it.");
            false
        };

        Ok(Self {
            version: 1,
            network_mode,
            support_ping,
            support_tuntap,
            packet_forwarding,
            enable_kvm,
            expose_pipewire,
        })
    }
}
