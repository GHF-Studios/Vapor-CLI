//! Clap commands for SDK toolchain workflows.

use clap::Subcommand;
use vapor_sdk_core as core;

#[derive(Subcommand)]
pub(super) enum ToolchainCommand {
    /// Show the pinned Vapor Rust toolchain, host support, and install state.
    Status,
    /// Download and install the pinned portable Vapor Rust toolchain.
    Install,
    /// Repair the pinned portable Vapor Rust toolchain.
    Repair,
}

impl ToolchainCommand {
    pub(super) fn into_core(self) -> core::ToolchainCommand {
        match self {
            Self::Status => core::ToolchainCommand::Status,
            Self::Install => core::ToolchainCommand::Install,
            Self::Repair => core::ToolchainCommand::Repair,
        }
    }
}
