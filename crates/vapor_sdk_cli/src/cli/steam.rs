//! Clap commands for SteamCMD tooling.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use vapor_sdk_core::steam::types::{
    SteamCommand as CoreSteamCommand, SteamLoginRequest, SteamStatusRequest,
};

#[derive(Subcommand)]
pub(super) enum SteamCommand {
    /// Inspect SteamCMD availability and Vapor Steam state paths.
    Status(SteamToolArgs),
    /// Run SteamCMD login without Vapor seeing the password or Steam Guard code.
    Login(SteamLoginArgs),
}

impl SteamCommand {
    pub(super) fn into_core(self) -> CoreSteamCommand {
        match self {
            Self::Status(args) => CoreSteamCommand::Status(args.into_core()),
            Self::Login(args) => CoreSteamCommand::Login(args.into_core()),
        }
    }
}

#[derive(Args)]
pub(super) struct SteamToolArgs {
    /// SteamCMD executable path. Defaults to resolving `steamcmd` through PATH.
    #[arg(long, default_value = "steamcmd", help_heading = "Steam")]
    steamcmd: PathBuf,
}

impl SteamToolArgs {
    fn into_core(self) -> SteamStatusRequest {
        SteamStatusRequest {
            steamcmd: self.steamcmd,
        }
    }
}

#[derive(Args)]
pub(super) struct SteamLoginArgs {
    /// Steam account used by SteamCMD.
    #[arg(long, help_heading = "Steam")]
    account: String,
    /// SteamCMD executable path. Defaults to resolving `steamcmd` through PATH.
    #[arg(long, default_value = "steamcmd", help_heading = "Steam")]
    steamcmd: PathBuf,
}

impl SteamLoginArgs {
    fn into_core(self) -> SteamLoginRequest {
        SteamLoginRequest {
            account: self.account,
            steamcmd: self.steamcmd,
        }
    }
}
