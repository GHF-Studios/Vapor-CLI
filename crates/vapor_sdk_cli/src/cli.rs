//! Clap parser and conversion into SDK-core command requests.

mod args;
mod leaf;
mod pack;
mod packagepack;
mod repair;
mod root;
mod steam;
mod template;
mod toolchain;

use clap::{Parser, Subcommand};
use leaf::LeafCommand;
use pack::PackCommand;
use packagepack::PackagepackCommand;
use repair::RepairCommand;
use root::RootCommand;
use std::path::PathBuf;
use steam::SteamCommand;
use template::TemplateCommand;
use toolchain::ToolchainCommand;
use vapor_sdk_core as core;

/// Parsed top-level SDK invocation.
#[derive(Parser)]
#[command(name = "vapor sdk")]
#[command(
    version,
    about = "Author Vapor workspaces, content, tools, and releases."
)]
#[command(
    long_about = "SDK CLI for Vapor authoring workflows. Use it to manage the portable toolchain, sync Vapor-managed workspaces, build through Vapor-owned Cargo, and prepare authored content for packaging and publishing."
)]
#[command(
    arg_required_else_help = true,
    subcommand_required = true,
    propagate_version = true
)]
pub(crate) struct Cli {
    /// Operate on a workspace/repo outside the current shell directory.
    #[arg(long, value_name = "PATH", help_heading = "Workspace Targeting")]
    workspace: Option<PathBuf>,
    /// Show operation planning, diagnostics, historical context, and live detail.
    #[arg(long, help_heading = "Output")]
    verbose: bool,
    /// Accept prompts automatically for commands that otherwise stop for confirmation.
    #[arg(long, help_heading = "Prompt Control")]
    yes: bool,
    /// Permit destructive or risk-bearing operations when that command supports it.
    #[arg(long, help_heading = "Prompt Control")]
    force: bool,
    /// Reject authoring mutations that would leave authored content invalid.
    #[arg(long, help_heading = "Prompt Control")]
    strict: bool,
    /// Keep old unused installed versions after update, lock, repair, or cleanup.
    #[arg(long, help_heading = "Prompt Control")]
    keep_unused_versions: bool,
    #[command(subcommand)]
    command: Command,
}

impl Cli {
    pub(crate) fn into_parts(self) -> (core::GlobalOptions, core::SdkCommand) {
        let globals = core::GlobalOptions {
            workspace: self.workspace,
            verbose: self.verbose,
            yes: self.yes,
            force: self.force,
            strict: self.strict,
            keep_unused_versions: self.keep_unused_versions,
        };
        (globals, self.command.into_core())
    }
}

/// Root SDK workflows.
#[derive(Subcommand)]
enum Command {
    /// Print SDK version/build identity.
    Version,
    /// Summarize SDK health and authoring environment state.
    Status,
    /// Show app-root shell activation and PATH setup.
    Env,
    /// Package or publish the first-party Steam-distributed Vapor root.
    Root {
        #[command(subcommand)]
        command: RootCommand,
    },
    /// Inspect or authenticate SteamCMD for Vapor release workflows.
    Steam {
        #[command(subcommand)]
        command: SteamCommand,
    },
    /// Run `cargo check` through the Vapor-managed Cargo binary.
    Check,
    /// Run `cargo fmt` through the Vapor-managed Cargo binary.
    Fmt,
    /// Run `cargo build --workspace` through the Vapor-managed Cargo binary.
    Build,
    /// Run `cargo test --workspace` through the Vapor-managed Cargo binary.
    Test,
    /// Build and promote the current first-party tool CLI into the executable-root `bin` directory.
    Deploy,
    /// Inspect or sync the current Vapor workspace.
    Workspace {
        #[command(subcommand)]
        command: WorkspaceManageCommand,
    },
    /// Inspect, plan, or apply repairs to SDK-managed state.
    Repair {
        #[command(subcommand)]
        command: RepairCommand,
    },
    /// Inspect, install, or repair the portable Vapor Rust toolchain.
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommand,
    },
    /// Inspect SDK templates and creation recipes.
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
    /// Author packagepacks, the root playable composition boundary.
    Packagepack {
        #[command(subcommand)]
        command: PackagepackCommand,
    },
    /// Author enginepacks containing engines, engine mods, and nested enginepacks.
    Enginepack {
        #[command(subcommand)]
        command: PackCommand,
    },
    /// Author gamepacks containing games, game mods, and nested gamepacks.
    Gamepack {
        #[command(subcommand)]
        command: PackCommand,
    },
    /// Author modpacks containing engine mods, game mods, extension mods, and nested modpacks.
    Modpack {
        #[command(subcommand)]
        command: PackCommand,
    },
    /// Author engine content.
    Engine {
        #[command(subcommand)]
        command: LeafCommand,
    },
    /// Author game content.
    Game {
        #[command(subcommand)]
        command: LeafCommand,
    },
    /// Author engine mod content.
    #[command(name = "engine-mod")]
    EngineMod {
        #[command(subcommand)]
        command: LeafCommand,
    },
    /// Author game mod content.
    #[command(name = "game-mod")]
    GameMod {
        #[command(subcommand)]
        command: LeafCommand,
    },
    /// Author extension mod content.
    #[command(name = "extension-mod")]
    ExtensionMod {
        #[command(subcommand)]
        command: LeafCommand,
    },
}

impl Command {
    fn into_core(self) -> core::SdkCommand {
        match self {
            Self::Version => core::SdkCommand::Version,
            Self::Status => core::SdkCommand::Status,
            Self::Env => core::SdkCommand::Environment(core::EnvironmentCommand::Status),
            Self::Root { command } => core::SdkCommand::Root(command.into_core()),
            Self::Steam { command } => core::SdkCommand::Steam(command.into_core()),
            Self::Check => core::SdkCommand::Workspace(core::WorkspaceCommand::Check),
            Self::Fmt => core::SdkCommand::Workspace(core::WorkspaceCommand::Fmt),
            Self::Build => core::SdkCommand::Workspace(core::WorkspaceCommand::Build),
            Self::Test => core::SdkCommand::Workspace(core::WorkspaceCommand::Test),
            Self::Deploy => core::SdkCommand::Workspace(core::WorkspaceCommand::Deploy),
            Self::Workspace { command } => core::SdkCommand::Workspace(command.into_core()),
            Self::Repair { command } => core::SdkCommand::Repair(command.into_core()),
            Self::Toolchain { command } => core::SdkCommand::Toolchain(command.into_core()),
            Self::Template { command } => core::SdkCommand::Template(command.into_core()),
            Self::Packagepack { command } => core::SdkCommand::Packagepack(command.into_core()),
            Self::Enginepack { command } => pack_command(core::ContentType::Enginepack, command),
            Self::Gamepack { command } => pack_command(core::ContentType::Gamepack, command),
            Self::Modpack { command } => pack_command(core::ContentType::Modpack, command),
            Self::Engine { command } => leaf_command(core::ContentType::Engine, command),
            Self::Game { command } => leaf_command(core::ContentType::Game, command),
            Self::EngineMod { command } => leaf_command(core::ContentType::EngineMod, command),
            Self::GameMod { command } => leaf_command(core::ContentType::GameMod, command),
            Self::ExtensionMod { command } => {
                leaf_command(core::ContentType::ExtensionMod, command)
            }
        }
    }
}

#[derive(Subcommand)]
enum WorkspaceManageCommand {
    /// Inspect the current Vapor workspace identity and managed structure.
    Status,
    /// Create or update SDK-managed workspace structure.
    Sync,
}

impl WorkspaceManageCommand {
    fn into_core(self) -> core::WorkspaceCommand {
        match self {
            Self::Status => core::WorkspaceCommand::Status,
            Self::Sync => core::WorkspaceCommand::Sync,
        }
    }
}

fn pack_command(pack_type: core::ContentType, command: PackCommand) -> core::SdkCommand {
    core::SdkCommand::Pack {
        pack_type,
        command: command.into_core(),
    }
}

fn leaf_command(content_type: core::ContentType, command: LeafCommand) -> core::SdkCommand {
    core::SdkCommand::Leaf {
        content_type,
        command: command.into_core(),
    }
}
