//! Clap commands for first-party root package and SteamPipe publishing.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use vapor_sdk_core::root::types::{
    ROOT_STEAM_APP_ID, ROOT_STEAM_DEPOT_ID, RootCommand as CoreRootCommand, RootPackageRequest,
    RootPublishRequest,
};

#[derive(Subcommand)]
pub(super) enum RootCommand {
    /// Assemble the Steam redistributable root package and SteamPipe VDFs.
    Package(RootPackageArgs),
    /// Assemble and upload the root package through SteamCMD.
    Publish(RootPublishArgs),
}

impl RootCommand {
    pub(super) fn into_core(self) -> CoreRootCommand {
        match self {
            Self::Package(args) => CoreRootCommand::Package(args.into_core()),
            Self::Publish(args) => CoreRootCommand::Publish(args.into_core()),
        }
    }
}

#[derive(Args)]
pub(super) struct RootPackageArgs {
    /// Show the package plan without writing package content or SteamPipe scripts.
    #[arg(long, help_heading = "Execution")]
    plan: bool,
    /// Steam AppID for the root Vapor/Loo Cast application.
    #[arg(long, default_value_t = ROOT_STEAM_APP_ID, help_heading = "Steam")]
    app_id: u32,
    /// Steam DepotID that receives the root app files.
    #[arg(long, default_value_t = ROOT_STEAM_DEPOT_ID, help_heading = "Steam")]
    depot_id: u32,
    /// Internal Steamworks build description.
    #[arg(
        long = "desc",
        default_value = "Vapor root package",
        help_heading = "Steam"
    )]
    description: String,
    /// Beta branch to set live after upload. The default branch must be set live in Steamworks.
    #[arg(long, help_heading = "Steam")]
    set_live: Option<String>,
}

impl RootPackageArgs {
    fn into_core(self) -> RootPackageRequest {
        RootPackageRequest {
            plan: self.plan,
            app_id: self.app_id,
            depot_id: self.depot_id,
            description: self.description,
            set_live: self.set_live,
        }
    }
}

#[derive(Args)]
pub(super) struct RootPublishArgs {
    /// Show the publish plan without writing package content or invoking SteamCMD.
    #[arg(long, help_heading = "Execution")]
    plan: bool,
    /// Steam AppID for the root Vapor/Loo Cast application.
    #[arg(long, default_value_t = ROOT_STEAM_APP_ID, help_heading = "Steam")]
    app_id: u32,
    /// Steam DepotID that receives the root app files.
    #[arg(long, default_value_t = ROOT_STEAM_DEPOT_ID, help_heading = "Steam")]
    depot_id: u32,
    /// Steam account used by SteamCMD.
    #[arg(long, help_heading = "Steam")]
    account: String,
    /// SteamCMD executable path. Defaults to resolving `steamcmd` through PATH.
    #[arg(long, default_value = "steamcmd", help_heading = "Steam")]
    steamcmd: PathBuf,
    /// Internal Steamworks build description.
    #[arg(
        long = "desc",
        default_value = "Vapor root package",
        help_heading = "Steam"
    )]
    description: String,
    /// Beta branch to set live after upload. The default branch must be set live in Steamworks.
    #[arg(long, help_heading = "Steam")]
    set_live: Option<String>,
}

impl RootPublishArgs {
    fn into_core(self) -> RootPublishRequest {
        RootPublishRequest {
            plan: self.plan,
            app_id: self.app_id,
            depot_id: self.depot_id,
            description: self.description,
            account: self.account,
            steamcmd: self.steamcmd,
            set_live: self.set_live,
        }
    }
}
