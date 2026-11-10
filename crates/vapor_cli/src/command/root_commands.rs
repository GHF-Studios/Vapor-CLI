//! Execution of application-root build, deployment, packaging, and publishing.

use super::client_commands::{
    connect_client_service, print_service_operation_message, send_client_action,
};
use super::content_commands::open_workspace_in_service;
use super::{resolve_publish_runtime_targets, resolve_runtime_targets, RootCommand, ShellState};
use crate::{
    app_local_tools::AppToolRequirement,
    distribution::StageOptions,
    metadata::{ResolvedMetadata, ValidationPlan},
    workflow::CargoWorkflow,
};
use std::time::{Duration, Instant};
use vapor_client_protocol::{ClientAction, ClientEvent, ProtocolFrame, PublishingAction};

pub(super) fn execute_root(command: RootCommand, state: &ShellState) -> Result<(), String> {
    let metadata = ResolvedMetadata::resolve(state);
    match command {
        RootCommand::Build {
            release,
            target,
            release_targets,
            host_only,
        } => {
            metadata.validate(
                &ValidationPlan::new("build the Vapor application")
                    .app_local_tools(&[AppToolRequirement::Rust])
                    .workspace(),
            )?;
            let workspace_manifest = metadata.workspace_manifest()?;
            let targets =
                resolve_runtime_targets(workspace_manifest, &target, release_targets, host_only)?;
            let workflow = if release {
                CargoWorkflow::BuildRelease
            } else {
                CargoWorkflow::Build
            };
            run_root_workflow_targets(state, workflow, &targets)?;
            println!(
                "hint: deploy built app outputs with `root deploy`, package with `root package`, or publish with `root publish`"
            );
        }
        RootCommand::Deploy {
            skip_docs,
            target,
            release_targets,
            host_only,
        } => {
            metadata.validate(
                &ValidationPlan::new("deploy the Vapor application locally")
                    .app_local_tools(&[AppToolRequirement::Rust])
                    .workspace(),
            )?;
            let workspace_manifest = metadata.workspace_manifest()?;
            let targets =
                resolve_runtime_targets(workspace_manifest, &target, release_targets, host_only)?;
            let mut client = connect_client_service(state)?;
            open_workspace_in_service(&mut client, state)?;
            let event = send_client_action(
                &mut client,
                ClientAction::Workspace(
                    vapor_client_protocol::WorkspaceAction::DeployApplication {
                        target_triples: targets.clone(),
                        skip_documentation: skip_docs,
                    },
                ),
            )?
            .ok_or_else(|| {
                "Vapor Client accepted application deployment without returning its completion"
                    .to_owned()
            })?;
            print_service_operation_message(Some(event));
            if metadata.distribution_manifest().is_ok() {
                println!(
                    "hint: package with `root package` or review Steam publication with `root publish --dry-run`"
                );
            } else {
                println!(
                    "hint: Vapor standalone releases are independent; Steam publication requires a valid depot policy on the selected source"
                );
            }
        }
        RootCommand::Package {
            target,
            release_targets,
            host_only,
        } => {
            metadata.validate(&ValidationPlan::new("package the Vapor application").workspace())?;
            let targets = resolve_runtime_targets(
                metadata.workspace_manifest()?,
                &target,
                release_targets,
                host_only,
            )?;
            let mut client = connect_client_service(state)?;
            open_workspace_in_service(&mut client, state)?;
            let event = send_client_action(
                &mut client,
                ClientAction::Workspace(
                    vapor_client_protocol::WorkspaceAction::PackageApplication {
                        target_triples: targets,
                    },
                ),
            )?
            .ok_or_else(|| {
                "Vapor Client accepted application packaging without returning its completion"
                    .to_owned()
            })?;
            print_service_operation_message(Some(event));
            if metadata.distribution_manifest().is_ok() {
                println!(
                    "hint: this stages standalone bundles and declared Steam depots; `root publish --dry-run` previews the Steam upload"
                );
            } else {
                println!(
                    "hint: this source has no valid Steam depot policy; Vapor standalone releases are handled independently"
                );
            }
        }
        RootCommand::Publish {
            account,
            branch,
            target,
            release_targets,
            host_only,
            description,
            dry_run,
            yes,
        } => {
            if !dry_run && !yes {
                return Err(
                    "real app publication requires --yes after reviewing a dry-run plan".to_owned(),
                );
            }
            if !dry_run && account.as_deref().is_none_or(str::is_empty) {
                return Err(
                    "real app publication requires --account ACCOUNT after reviewing --dry-run"
                        .to_owned(),
                );
            }
            let tool_requirements = if dry_run {
                Vec::new()
            } else {
                vec![AppToolRequirement::SteamCmd]
            };
            metadata.validate(
                &ValidationPlan::new("publish manifest-declared application content")
                    .app_local_tools(&tool_requirements)
                    .workspace()
                    .distribution(),
            )?;
            let targets = resolve_publish_runtime_targets(
                metadata.workspace_manifest()?,
                &target,
                release_targets,
                host_only,
                dry_run,
                "app publication",
            )?;
            let stage_options = StageOptions::runtime().with_runtime_targets(targets.clone());
            let mut client = connect_client_service(state)?;
            crate::diagnostics::event("application release: opening selected source workspace");
            open_workspace_in_service(&mut client, state)?;
            crate::diagnostics::event(
                "application release: source workspace opened; requesting manifest staging",
            );
            let stage_event = send_client_action(
                &mut client,
                ClientAction::Publishing(PublishingAction::StageApplication {
                    account: account.clone().unwrap_or_else(|| "dry-run".to_owned()),
                    branch: branch.clone(),
                    description: description.clone(),
                    runtime_targets: targets.clone(),
                }),
            )?
            .ok_or_else(|| {
                "Vapor Client accepted application release staging without returning its completion"
                    .to_owned()
            })?;
            crate::diagnostics::event(format!(
                "root publish: release stage operation {} completed; awaiting its review snapshot",
                stage_event.id
            ));
            let review = wait_for_application_release_review(&mut client, &stage_event.id)?;
            print_service_operation_message(Some(stage_event));
            println!("payload: manifest-declared application content");
            println!(
                "ABI targets: {}",
                stage_options.runtime_targets().join(", ")
            );
            println!("app id: {}", review.app_id);
            println!("depot ids: {}", display_depot_ids(&review.depot_ids));
            println!("staged files: {}", review.files);
            println!("payload fingerprint: {}", review.fingerprint);
            println!("SteamPipe build script: {}", review.build_script);
            if dry_run {
                println!(
                    "hint: review the staged content, then run `root publish --account ACCOUNT --yes`"
                );
            } else {
                let confirmation =
                    format!("PUBLISH APPLICATION {} TO {}", review.app_id, review.branch);
                let event = send_client_action(
                    &mut client,
                    ClientAction::Publishing(PublishingAction::PublishApplication {
                        review_id: review.review_id,
                        confirmation,
                    }),
                )?;
                print_service_operation_message(event);
                println!("hint: Steam accepted the app upload; verify the target beta branch");
            }
        }
    }
    Ok(())
}

fn display_depot_ids(depot_ids: &[u32]) -> String {
    depot_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn wait_for_application_release_review(
    client: &mut vapor_client::Client,
    operation_id: &str,
) -> Result<vapor_client_protocol::ApplicationReleaseSnapshot, String> {
    const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
    let deadline = Instant::now() + SNAPSHOT_TIMEOUT;
    loop {
        if let Some(review) = client
            .latest_snapshot()
            .application_release
            .as_ref()
            .filter(|review| review.review_id == operation_id && review.planned)
        {
            return Ok(review.clone());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(
                "Vapor Client completed release staging without returning its review snapshot"
                    .to_owned(),
            );
        }
        match client
            .recv_timeout(remaining)
            .map_err(|error| error.to_string())?
        {
            ProtocolFrame::Event(ClientEvent::ProtocolError(error)) => return Err(error.message),
            ProtocolFrame::Event(_) | ProtocolFrame::Response(_) => {}
            _ => {
                return Err(
                    "Vapor Client sent an unexpected frame while returning the release review"
                        .to_owned(),
                );
            }
        }
    }
}

fn run_root_workflow_targets(
    state: &ShellState,
    workflow: CargoWorkflow,
    targets: &[String],
) -> Result<(), String> {
    super::execute_workspace_tool(
        state,
        workflow,
        vapor_client_protocol::WorkspaceProjectSelection::Installable,
        targets,
    )
    .map(|_| ())
}
