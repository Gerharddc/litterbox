use inquire_derive::Selectable;
use std::fmt::Display;

#[derive(Debug, Copy, Clone, Selectable)]
pub enum Template {
    OpenSuseTumbleweed,
    UbuntuLts,
    CachyOS,
}

impl Template {
    pub fn contents(&self) -> &'static str {
        match self {
            Template::OpenSuseTumbleweed => include_str!("../templates/tumbleweed.Dockerfile"),
            Template::UbuntuLts => include_str!("../templates/ubuntu-latest.Dockerfile"),
            Template::CachyOS => include_str!("../templates/cachyos.Dockerfile"),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Template::OpenSuseTumbleweed => "OpenSUSE Tumbleweed",
            Template::UbuntuLts => "Ubuntu LTS",
            Template::CachyOS => "CachyOS",
        }
    }
}

impl Display for Template {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}
