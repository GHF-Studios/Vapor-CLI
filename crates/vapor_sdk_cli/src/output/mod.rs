//! Output for parsed SDK commands.

mod common;
mod environment;
mod root;
mod steam;
mod stub;
mod toolchain;
mod workspace;

use vapor_sdk_core::root::package::root_package;
use vapor_sdk_core::root::publish::root_publish;
use vapor_sdk_core::root::types::RootCommand;
use vapor_sdk_core::steam::process::steam_login;
use vapor_sdk_core::steam::status::steam_status;
use vapor_sdk_core::steam::types::SteamCommand;
use vapor_sdk_core::{
    EnvironmentCommand, GlobalOptions, SdkCommand, ToolchainCommand, WorkspaceCommand,
    environment_status, toolchain_install, toolchain_status, workspace_build, workspace_check,
    workspace_deploy, workspace_fmt, workspace_status, workspace_sync,
};

pub(crate) fn print_command(
    globals: GlobalOptions,
    command: &SdkCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let spec = vapor_sdk_core::describe_command(command);
    crate::safety::guard(&globals, command, &spec)?;

    match command {
        SdkCommand::Environment(EnvironmentCommand::Status) => {
            environment::print_status(&globals, &spec, environment_status()?)
        }
        SdkCommand::Workspace(WorkspaceCommand::Status) => {
            workspace::print_status(&globals, &spec, workspace_status(&globals)?)
        }
        SdkCommand::Workspace(WorkspaceCommand::Sync) => {
            workspace::print_sync(&globals, &spec, workspace_sync(&globals)?)
        }
        SdkCommand::Workspace(WorkspaceCommand::Check) => {
            workspace::print_cargo(&globals, &spec, workspace_check(&globals)?)
        }
        SdkCommand::Workspace(WorkspaceCommand::Fmt) => {
            workspace::print_cargo(&globals, &spec, workspace_fmt(&globals)?)
        }
        SdkCommand::Workspace(WorkspaceCommand::Build) => {
            workspace::print_cargo(&globals, &spec, workspace_build(&globals)?)
        }
        SdkCommand::Workspace(WorkspaceCommand::Deploy) => {
            workspace::print_deploy(&globals, &spec, workspace_deploy(&globals)?)
        }
        SdkCommand::Root(RootCommand::Package(request)) => {
            root::print_package(&globals, &spec, root_package(request)?)
        }
        SdkCommand::Root(RootCommand::Publish(request)) => {
            root::print_publish(&globals, &spec, root_publish(request)?)
        }
        SdkCommand::Steam(SteamCommand::Status(request)) => {
            steam::print_status(&globals, &spec, steam_status(request)?)
        }
        SdkCommand::Steam(SteamCommand::Login(request)) => {
            steam::print_login(&globals, &spec, steam_login(request)?)
        }
        SdkCommand::Toolchain(ToolchainCommand::Status) => {
            toolchain::print_status(&globals, &spec, toolchain_status()?)
        }
        SdkCommand::Toolchain(ToolchainCommand::Install) => {
            toolchain::print_install(&globals, &spec, toolchain_install()?)
        }
        _ => stub::print(&globals, &spec),
    }
}
