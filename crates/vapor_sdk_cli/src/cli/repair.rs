//! Clap commands for SDK repair workflows.

use clap::{Subcommand, ValueEnum};
use vapor_sdk_core as core;

#[derive(Subcommand)]
pub(super) enum RepairCommand {
    Status,
    Plan { target: RepairTarget },
    Apply { target: RepairTarget },
}

impl RepairCommand {
    pub(super) fn into_core(self) -> core::RepairCommand {
        match self {
            Self::Status => core::RepairCommand::Status,
            Self::Plan { target } => core::RepairCommand::Plan {
                target: target.into(),
            },
            Self::Apply { target } => core::RepairCommand::Apply {
                target: target.into(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(super) enum RepairTarget {
    #[value(name = "core_state")]
    CoreState,
    Toolchain,
    Steam,
    #[value(name = "content_catalog")]
    ContentCatalog,
    #[value(name = "content_library")]
    ContentLibrary,
    #[value(name = "active_composition")]
    ActiveComposition,
    All,
}

impl From<RepairTarget> for core::RepairTarget {
    fn from(value: RepairTarget) -> Self {
        match value {
            RepairTarget::CoreState => Self::CoreState,
            RepairTarget::Toolchain => Self::Toolchain,
            RepairTarget::Steam => Self::Steam,
            RepairTarget::ContentCatalog => Self::ContentCatalog,
            RepairTarget::ContentLibrary => Self::ContentLibrary,
            RepairTarget::ActiveComposition => Self::ActiveComposition,
            RepairTarget::All => Self::All,
        }
    }
}
