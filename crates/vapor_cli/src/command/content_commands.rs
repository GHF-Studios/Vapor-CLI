//! Execution of typed content, Library, and Workshop shell commands.

use super::client_commands::{
    acquire_workshop_items, active_source_root, connect_client_service, is_workshop_id,
    print_service_operation_message, require_workshop_id, send_client_action,
    send_client_action_with_authoring_result, set_active_composition_root,
};
use super::{
    CargoWorkflow, ContentCommand, ShellState, content, execute_workspace_tool,
    resolve_publish_runtime_targets, resolve_runtime_targets,
};
use crate::{
    app_local_tools::AppToolRequirement,
    metadata::{ResolvedMetadata, ValidationPlan},
};
use std::time::{Duration, Instant};
use vapor_client::{Client, ClientError};
use vapor_client_protocol::{
    AuthoringAction, AuthoringResultKind, CatalogAction, CatalogEntry, CatalogItemRole,
    CatalogLocationKind, ClientAction, ClientEvent, ClientSnapshot, LibraryAction, ProtocolFrame,
    WorkshopAction, WorkspaceAction, WorkspaceProjectSelection,
};

pub(super) fn execute_content(command: ContentCommand, state: &ShellState) -> Result<(), String> {
    execute_content_body(command, state)
}

fn execute_content_body(command: ContentCommand, state: &ShellState) -> Result<(), String> {
    match command {
        ContentCommand::Status => {
            let layout = content::ContentLayout::new(state.installation());
            println!("Content Status");
            println!();
            println!("Status");
            if let Some(content) = state.content() {
                println!("  Current content: {}", content.id());
                println!("    kind: {}", content.kind());
                println!("    root: {}", content.root().display());
            } else {
                println!("  Current content: none selected by directory");
            }
            println!("  Installed content root: {}", layout.installed().display());
            println!("  Content cache: {}", layout.cache().display());
            println!("  Content state: {}", layout.state().display());
            println!();
            println!("Next");
            println!("  content list");
        }
        ContentCommand::List => {
            let mut client = connect_client_service(state)?;
            if let Some(root) = active_source_root(state) {
                send_client_action(
                    &mut client,
                    ClientAction::Workspace(WorkspaceAction::Open { root }),
                )?;
            }
            send_client_action(&mut client, ClientAction::Catalog(CatalogAction::Refresh))?;
            let snapshot = client.latest_snapshot();
            if let Some(error) = &snapshot.catalog_status.error {
                return Err(format!(
                    "Vapor Client could not refresh the content Library: {error}"
                ));
            }
            if !snapshot.catalog_status.loaded {
                return Err(
                    "Vapor Client did not return a completed content Library refresh".to_owned(),
                );
            }
            println!("Content Library");
            if snapshot.catalog_status.omitted > 0 {
                println!(
                    "  {} item(s) omitted because the service catalog reached its display limit",
                    snapshot.catalog_status.omitted
                );
            }
            for (title, kind) in [
                ("Workspace Content", CatalogLocationKind::Workspace),
                ("Downloaded Content", CatalogLocationKind::Cache),
                ("Installed Content", CatalogLocationKind::Installed),
            ] {
                let items = snapshot
                    .catalog
                    .iter()
                    .filter(|entry| entry.locations.iter().any(|location| location.kind == kind))
                    .collect::<Vec<_>>();
                println!("\n{title} ({} item(s))", items.len());
                if items.is_empty() {
                    println!("  none");
                } else {
                    for item in items {
                        print_catalog_item(item, kind);
                    }
                }
            }
            println!("\nNext");
            if snapshot.catalog.iter().any(|entry| {
                entry
                    .locations
                    .iter()
                    .any(|location| location.kind == CatalogLocationKind::Workspace)
            }) {
                println!("  content validate");
            } else {
                println!("  source open /path/to/content-workspace");
            }
        }
        ContentCommand::Validate { artifact } => {
            let workspace_root = state.active_paths()?.source().root().to_path_buf();
            let mut client = connect_client_service(state)?;
            send_client_action(
                &mut client,
                ClientAction::Workspace(WorkspaceAction::Open {
                    root: workspace_root.display().to_string(),
                }),
            )?;
            let event = send_client_action(
                &mut client,
                ClientAction::Library(LibraryAction::ValidateArtifacts {
                    selector: artifact,
                    workspace_root: Some(workspace_root.display().to_string()),
                }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Build {
            target,
            release_targets,
            host_only,
        } => {
            let metadata = ResolvedMetadata::resolve(state);
            metadata.validate(
                &ValidationPlan::new("build content")
                    .app_local_tools(&[AppToolRequirement::Rust])
                    .workspace(),
            )?;
            let targets = resolve_runtime_targets(
                metadata.workspace_manifest()?,
                &target,
                release_targets,
                host_only,
            )?;
            execute_workspace_tool(
                state,
                CargoWorkflow::Build,
                WorkspaceProjectSelection::All,
                &targets,
            )?;
            println!("hint: stage content with `content package ARTIFACT`");
        }
        ContentCommand::Deploy {
            artifact,
            select,
            target,
            release_targets,
            host_only,
        } => {
            let metadata = ResolvedMetadata::resolve(state);
            let targets = resolve_runtime_targets(
                metadata.workspace_manifest()?,
                &target,
                release_targets,
                host_only,
            )?;
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Install {
                    selector: artifact.clone(),
                    account: None,
                    workspace_root: active_source_root(state),
                    target_triples: targets,
                }),
            )?;
            print_service_operation_message(event);
            if select {
                let installed = resolve_installed_packagepack(state, &artifact)?;
                set_active_composition_root(
                    state,
                    Some((installed.0.as_str(), installed.1.as_deref())),
                )?;
                println!(
                    "selected packagepack: {}{}",
                    installed.0,
                    installed
                        .1
                        .map(|version| format!("@{version}"))
                        .unwrap_or_default()
                );
            } else {
                println!(
                    "hint: select a packagepack with `content select ARTIFACT_OR_WORKSHOP_ID`"
                );
            }
        }
        ContentCommand::Package {
            artifact,
            target,
            release_targets,
            host_only,
            dry_run,
        } => {
            let metadata = ResolvedMetadata::resolve(state);
            let targets = resolve_runtime_targets(
                metadata.workspace_manifest()?,
                &target,
                release_targets,
                host_only,
            )?;
            let workspace_root = state.active_paths()?.source().root().to_path_buf();
            let mut client = connect_client_service(state)?;
            send_client_action(
                &mut client,
                ClientAction::Workspace(WorkspaceAction::Open {
                    root: workspace_root.display().to_string(),
                }),
            )?;
            let (_, result) = send_client_action_with_authoring_result(
                &mut client,
                ClientAction::Authoring(AuthoringAction::PackageContent {
                    selector: artifact.clone(),
                    target_triples: targets,
                    dry_run,
                }),
            )?;
            let Some(AuthoringResultKind::ContentPackaged {
                id,
                root,
                target,
                fingerprint,
                files,
                bytes,
                dry_run: report_is_dry_run,
                receipt,
            }) = result
            else {
                return Err(
                    "Vapor Client completed packaging without returning its report.".to_owned(),
                );
            };
            if dry_run != report_is_dry_run {
                return Err(
                    "Vapor Client returned a package result that does not match the requested mode."
                        .to_owned(),
                );
            }
            println!(
                "{}: {id}",
                if report_is_dry_run {
                    "would package"
                } else {
                    "packaged"
                }
            );
            println!("  artifact root: {root}");
            println!("  runtime target: {target}");
            println!("  fingerprint: {fingerprint} ({files} files, {bytes} bytes)");
            if let Some(receipt) = receipt {
                println!("  receipt: {receipt}");
            }
            if dry_run {
                println!("dry-run: no package files were changed");
            } else {
                println!(
                    "hint: cache it with `content acquire {artifact}` or preview Workshop upload with `content publish {artifact} --dry-run`"
                );
            }
        }
        ContentCommand::Acquire { target } => {
            if is_workshop_id(&target) {
                acquire_workshop_items(state, vec![target])?;
            } else {
                let event = send_client_action(
                    &mut connect_client_service(state)?,
                    ClientAction::Library(LibraryAction::Acquire {
                        selectors: vec![target],
                        workspace_root: active_source_root(state),
                    }),
                )?;
                print_service_operation_message(event);
            }
        }
        ContentCommand::Subscribe { target } => {
            require_workshop_id(&target)?;
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Workshop(WorkshopAction::Subscribe {
                    item_ids: vec![target.clone()],
                }),
            )?;
            println!("Subscribed to Workshop item {target} and requested it for the Library.");
            print_service_operation_message(event);
        }
        ContentCommand::Download { targets } => {
            let workshop_ids = targets
                .iter()
                .filter(|target| is_workshop_id(target))
                .count();
            if workshop_ids == targets.len() {
                acquire_workshop_items(state, targets)?;
            } else if workshop_ids > 0 {
                return Err(
                    "download authored content and Workshop items in separate commands so each uses its owning provider".to_owned(),
                );
            } else {
                let event = send_client_action(
                    &mut connect_client_service(state)?,
                    ClientAction::Library(LibraryAction::Acquire {
                        selectors: targets,
                        workspace_root: active_source_root(state),
                    }),
                )?;
                print_service_operation_message(event);
            }
        }
        ContentCommand::Install { target, account } => {
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Install {
                    selector: target,
                    account,
                    workspace_root: active_source_root(state),
                    target_triples: Vec::new(),
                }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Update { target } => {
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Update {
                    selector: target,
                    workspace_root: active_source_root(state),
                }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Verify { target } => {
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Verify { selector: target }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Selected => {
            let client = connect_client_service(state)?;
            let snapshot = client.latest_snapshot();
            let active_id = snapshot
                .active_composition_id
                .as_deref()
                .unwrap_or(&snapshot.durable_state.active_composition_id);
            let composition = snapshot
                .durable_state
                .compositions
                .compositions
                .iter()
                .find(|composition| composition.id == active_id);
            match composition.and_then(|composition| composition.root_package.as_ref()) {
                Some(root) => println!(
                    "selected packagepack: {}{}",
                    root.id,
                    root.version
                        .as_deref()
                        .map(|version| format!("@{version}"))
                        .unwrap_or_default()
                ),
                None => {
                    println!("selected packagepack: none");
                    println!("hint: select one with `content select ARTIFACT`");
                }
            }
        }
        ContentCommand::Select { target } => {
            let installed = resolve_installed_packagepack(state, &target)?;
            set_active_composition_root(
                state,
                Some((installed.0.as_str(), installed.1.as_deref())),
            )?;
            println!(
                "selected packagepack: {}{}",
                installed.0,
                installed
                    .1
                    .map(|version| format!("@{version}"))
                    .unwrap_or_default()
            );
        }
        ContentCommand::Deselect => {
            set_active_composition_root(state, None)?;
            println!("selected packagepack: none");
        }
        ContentCommand::Repair { target } => {
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Repair {
                    selector: target,
                    workspace_root: active_source_root(state),
                }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Disable { target } => {
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Disable { selector: target }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Enable { target } => {
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Enable { selector: target }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Uninstall { target } => {
            let event = send_client_action(
                &mut connect_client_service(state)?,
                ClientAction::Library(LibraryAction::Uninstall { selector: target }),
            )?;
            print_service_operation_message(event);
        }
        ContentCommand::Create {
            artifact,
            account,
            target,
            release_targets,
            host_only,
            dry_run,
            yes,
        } => {
            if !dry_run
                && account
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err(
                    "real Workshop creation requires --account ACCOUNT after reviewing --dry-run"
                        .to_owned(),
                );
            }
            if !dry_run && !yes {
                return Err(
                    "real Workshop creation requires --yes after reviewing --dry-run".to_owned(),
                );
            }
            let metadata = ResolvedMetadata::resolve(state);
            if !dry_run {
                metadata.validate(
                    &ValidationPlan::new("create Workshop item")
                        .app_local_tools(&[
                            AppToolRequirement::Rust,
                            AppToolRequirement::CrossToolchains,
                            AppToolRequirement::SteamCmd,
                        ])
                        .workspace(),
                )?;
            }
            let targets = resolve_publish_runtime_targets(
                metadata.workspace_manifest()?,
                &target,
                release_targets,
                host_only,
                dry_run,
                "Workshop item creation",
            )?;
            if dry_run {
                let review = stage_workshop_item_through_service(state, &artifact, &targets)?;
                if let Some(published_file_id) = review.preview.published_file_id.as_deref() {
                    return Err(format!(
                        "{} already has Workshop item {published_file_id}; use `content publish` to update it",
                        review.preview.id
                    ));
                }
                print_workshop_review(&review, "Create");
                println!(
                    "hint: after reviewing the staged package, run `content create {} --account ACCOUNT --yes` to publish it",
                    review.preview.id
                );
            } else {
                publish_workshop_item_through_service(
                    state,
                    &artifact,
                    &targets,
                    account.as_deref(),
                    None,
                    yes,
                    "creation",
                )?;
            }
        }
        ContentCommand::Publish {
            artifacts,
            account,
            target,
            release_targets,
            host_only,
            change_note,
            dry_run,
            yes,
        } => {
            if !dry_run
                && account
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err(
                    "real Workshop publication requires --account ACCOUNT after reviewing --dry-run"
                        .to_owned(),
                );
            }
            if !dry_run && !yes {
                return Err(
                    "real Workshop publication requires --yes after reviewing --dry-run".to_owned(),
                );
            }
            let metadata = ResolvedMetadata::resolve(state);
            if !dry_run {
                metadata.validate(
                    &ValidationPlan::new("publish Workshop items")
                        .app_local_tools(&[
                            AppToolRequirement::Rust,
                            AppToolRequirement::CrossToolchains,
                            AppToolRequirement::SteamCmd,
                        ])
                        .workspace(),
                )?;
            }
            let targets = resolve_publish_runtime_targets(
                metadata.workspace_manifest()?,
                &target,
                release_targets,
                host_only,
                dry_run,
                "Workshop publication",
            )?;
            if artifacts.len() == 1 && dry_run {
                let review = stage_workshop_item_through_service(state, &artifacts[0], &targets)?;
                validate_workshop_intent(
                    &review.preview.id,
                    review.preview.published_file_id.as_deref(),
                    "publication",
                )?;
                print_workshop_review(&review, "Publish update");
                println!(
                    "hint: after reviewing the staged package, run `content publish {} --account ACCOUNT --yes`",
                    review.preview.id
                );
            } else if !dry_run && artifacts.len() == 1 {
                publish_workshop_item_through_service(
                    state,
                    &artifacts[0],
                    &targets,
                    account.as_deref(),
                    change_note.as_deref(),
                    yes,
                    "publication",
                )?;
            } else if dry_run {
                let review = stage_workshop_batch_through_service(state, &artifacts, &targets)?;
                for item in &review.items {
                    validate_workshop_intent(
                        &item.preview.id,
                        item.preview.published_file_id.as_deref(),
                        "publication",
                    )?;
                    print_workshop_review_item(item, "Publish update");
                }
                println!("dry-run: no Workshop upload was performed");
                println!(
                    "hint: review all staged packages, then run `content publish ARTIFACT... --account ACCOUNT --yes` to publish the reviewed batch"
                );
            } else {
                publish_workshop_batch_through_service(
                    state,
                    &artifacts,
                    &targets,
                    account.as_deref(),
                    change_note.as_deref(),
                    yes,
                )?;
                println!(
                    "hint: verify the Workshop items in Steam and run `content acquire`/`content install` for a local roundtrip"
                );
            }
        }
        ContentCommand::Delete {
            target,
            dry_run,
            yes,
        } => {
            let report =
                content::delete_workshop_item(state.installation(), &target, dry_run, yes)?;
            println!("Workshop delete: {}", report.artifact_id());
            println!("  receipt: {}", report.receipt().display());
            if dry_run {
                println!("dry-run: no Workshop item was deleted");
            }
        }
    }
    Ok(())
}

fn publish_workshop_item_through_service(
    state: &ShellState,
    selector: &str,
    targets: &[String],
    account: Option<&str>,
    change_note: Option<&str>,
    confirmed: bool,
    operation: &str,
) -> Result<(), String> {
    let account = account
        .filter(|account| !account.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "real Workshop {operation} requires --account ACCOUNT after reviewing --dry-run"
            )
        })?;
    if !confirmed {
        return Err(format!(
            "real Workshop {operation} requires --yes after reviewing --dry-run"
        ));
    }

    let mut client = connect_client_service(state)?;
    open_workspace_in_service(&mut client, state)?;
    let review = stage_workshop_item_on_client(&mut client, selector, targets)?;
    validate_workshop_intent(
        &review.preview.id,
        review.preview.published_file_id.as_deref(),
        operation,
    )?;
    let confirmation = format!("PUBLISH {} TO {}", review.preview.id, account.trim());
    let event = send_client_action(
        &mut client,
        ClientAction::Workshop(WorkshopAction::Publish {
            review_id: review.review_id,
            account: account.to_owned(),
            confirmation,
            change_note: change_note.unwrap_or_default().to_owned(),
        }),
    )?;
    print_service_operation_message(event);
    Ok(())
}

fn stage_workshop_item_through_service(
    state: &ShellState,
    selector: &str,
    targets: &[String],
) -> Result<vapor_client_protocol::WorkshopReviewSnapshot, String> {
    let mut client = connect_client_service(state)?;
    open_workspace_in_service(&mut client, state)?;
    stage_workshop_item_on_client(&mut client, selector, targets)
}

pub(super) fn open_workspace_in_service(
    client: &mut Client,
    state: &ShellState,
) -> Result<(), String> {
    let source_root = state.active_paths()?.source().root().to_path_buf();
    send_client_action(
        client,
        ClientAction::Workspace(WorkspaceAction::Open {
            root: source_root.display().to_string(),
        }),
    )?;
    Ok(())
}

fn stage_workshop_item_on_client(
    client: &mut Client,
    selector: &str,
    targets: &[String],
) -> Result<vapor_client_protocol::WorkshopReviewSnapshot, String> {
    let stage_event = send_client_action(
        client,
        ClientAction::Workshop(WorkshopAction::Stage {
            selector: selector.to_owned(),
            target_triples: targets.to_vec(),
        }),
    )?
    .ok_or_else(|| "Vapor Client accepted staging without returning its operation.".to_owned())?;
    let operation_id = stage_event.id.clone();
    print_service_operation_message(Some(stage_event));
    wait_for_workshop_review(client, &operation_id, selector)
}

fn wait_for_workshop_review(
    client: &mut Client,
    operation_id: &str,
    selector: &str,
) -> Result<vapor_client_protocol::WorkshopReviewSnapshot, String> {
    wait_for_workshop_snapshot(client, "staging", |snapshot| {
        snapshot
            .workshop
            .review
            .as_ref()
            .filter(|review| {
                staged_workshop_review_matches(
                    &review.review_id,
                    &review.selector,
                    operation_id,
                    selector,
                )
            })
            .cloned()
    })
}

fn stage_workshop_batch_through_service(
    state: &ShellState,
    selectors: &[String],
    targets: &[String],
) -> Result<vapor_client_protocol::WorkshopBatchReviewSnapshot, String> {
    let mut client = connect_client_service(state)?;
    open_workspace_in_service(&mut client, state)?;
    stage_workshop_batch_on_client(&mut client, selectors, targets)
}

fn stage_workshop_batch_on_client(
    client: &mut Client,
    selectors: &[String],
    targets: &[String],
) -> Result<vapor_client_protocol::WorkshopBatchReviewSnapshot, String> {
    let event = send_client_action(
        client,
        ClientAction::Workshop(WorkshopAction::StageBatch {
            selectors: selectors.to_vec(),
            target_triples: targets.to_vec(),
        }),
    )?
    .ok_or_else(|| {
        "Vapor Client accepted batch staging without returning its operation.".to_owned()
    })?;
    let operation_id = event.id.clone();
    print_service_operation_message(Some(event));
    wait_for_workshop_batch_review(client, &operation_id, selectors)
}

fn wait_for_workshop_batch_review(
    client: &mut Client,
    operation_id: &str,
    selectors: &[String],
) -> Result<vapor_client_protocol::WorkshopBatchReviewSnapshot, String> {
    wait_for_workshop_snapshot(client, "batch staging", |snapshot| {
        snapshot
            .workshop
            .batch_review
            .as_ref()
            .filter(|review| {
                review.review_id == operation_id
                    && review
                        .items
                        .iter()
                        .map(|item| item.selector.as_str())
                        .eq(selectors.iter().map(String::as_str))
            })
            .cloned()
    })
}

fn wait_for_workshop_snapshot<T>(
    client: &mut Client,
    action: &str,
    mut select_review: impl FnMut(&ClientSnapshot) -> Option<T>,
) -> Result<T, String> {
    const SNAPSHOT_TIMEOUT: Duration = Duration::from_secs(5);
    const POLL_INTERVAL: Duration = Duration::from_millis(100);
    let deadline = Instant::now() + SNAPSHOT_TIMEOUT;

    loop {
        if let Some(review) = select_review(client.latest_snapshot()) {
            return Ok(review);
        }

        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!(
                "Vapor Client completed {action} but did not publish the matching review snapshot within five seconds."
            ));
        }
        match client.recv_timeout(remaining.min(POLL_INTERVAL)) {
            Ok(ProtocolFrame::Event(ClientEvent::ProtocolError(error))) => {
                return Err(error.message);
            }
            Ok(_) => {}
            Err(ClientError::Timeout(_)) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
}

fn staged_workshop_review_matches(
    review_id: &str,
    review_selector: &str,
    operation_id: &str,
    selector: &str,
) -> bool {
    review_id == operation_id && review_selector == selector
}

fn publish_workshop_batch_through_service(
    state: &ShellState,
    selectors: &[String],
    targets: &[String],
    account: Option<&str>,
    change_note: Option<&str>,
    confirmed: bool,
) -> Result<(), String> {
    let account = account
        .filter(|account| !account.trim().is_empty())
        .ok_or_else(|| "real Workshop publication requires --account ACCOUNT".to_owned())?;
    if !confirmed {
        return Err(
            "real Workshop publication requires --yes after reviewing --dry-run".to_owned(),
        );
    }
    let mut client = connect_client_service(state)?;
    open_workspace_in_service(&mut client, state)?;
    let review = stage_workshop_batch_on_client(&mut client, selectors, targets)?;
    for item in &review.items {
        validate_workshop_intent(
            &item.preview.id,
            item.preview.published_file_id.as_deref(),
            "publication",
        )?;
    }
    let item_label = if review.items.len() == 1 {
        "ITEM"
    } else {
        "ITEMS"
    };
    let confirmation = format!(
        "PUBLISH {} {item_label} TO {}",
        review.items.len(),
        account.trim()
    );
    let event = send_client_action(
        &mut client,
        ClientAction::Workshop(WorkshopAction::PublishBatch {
            review_id: review.review_id,
            account: account.to_owned(),
            confirmation,
            change_note: change_note.unwrap_or_default().to_owned(),
        }),
    )?;
    print_service_operation_message(event);
    Ok(())
}

fn print_workshop_review(review: &vapor_client_protocol::WorkshopReviewSnapshot, intent: &str) {
    print_workshop_review_item(
        &vapor_client_protocol::WorkshopReviewItemSnapshot {
            selector: review.selector.clone(),
            preview: review.preview.clone(),
        },
        intent,
    );
}

fn print_workshop_review_item(
    review: &vapor_client_protocol::WorkshopReviewItemSnapshot,
    intent: &str,
) {
    let preview = &review.preview;
    let version = preview
        .version
        .as_deref()
        .map(|version| format!("@{version}"))
        .unwrap_or_default();
    println!("{intent} Workshop package: {}{version}", preview.id);
    println!(
        "  title: {}",
        preview.title.as_deref().unwrap_or("(untitled)")
    );
    println!(
        "  visibility: {}",
        preview.visibility.as_deref().unwrap_or("(unset)")
    );
    println!(
        "  Workshop item: {}",
        preview.published_file_id.as_deref().unwrap_or("new item")
    );
    println!("  targets: {}", preview.target);
    println!("  package: {}", preview.root);
    println!(
        "  fingerprint: {} ({} files, {} bytes)",
        preview.fingerprint, preview.files, preview.bytes
    );
    if !preview.dependencies.is_empty() {
        println!("  dependencies:");
        for dependency in &preview.dependencies {
            println!("    {dependency}");
        }
    }
    if !preview.file_paths.is_empty() {
        println!("  files:");
        for path in &preview.file_paths {
            println!("    {path}");
        }
    }
    if preview.omitted_file_paths > 0 {
        println!("  {} more file(s) omitted", preview.omitted_file_paths);
    }
    println!("dry-run: no Workshop upload was performed");
}

fn validate_workshop_intent(
    artifact_id: &str,
    published_file_id: Option<&str>,
    intent: &str,
) -> Result<(), String> {
    match (intent, published_file_id) {
        ("creation", None) | ("publication", Some(_)) => Ok(()),
        ("creation", Some(id)) => Err(format!(
            "{artifact_id} already has Workshop item {id}; use `content publish` to update it"
        )),
        ("publication", None) => Err(format!(
            "{artifact_id} has no PublishedFileId; use `content create` to create its first Workshop item"
        )),
        _ => Err(format!("unsupported Workshop operation intent: {intent}")),
    }
}

fn resolve_installed_packagepack(
    state: &ShellState,
    selector: &str,
) -> Result<(String, Option<String>), String> {
    let mut client = connect_client_service(state)?;
    send_client_action(&mut client, ClientAction::Catalog(CatalogAction::Refresh))?;
    let (id, requested_version) = selector
        .split_once('@')
        .map(|(id, version)| (id, Some(version)))
        .unwrap_or((selector, None));
    let entry = client
        .latest_snapshot()
        .catalog
        .iter()
        .find(|entry| {
            (entry.id == id
                || entry.published_file_id.as_deref() == Some(selector)
                || entry.name.as_deref() == Some(selector))
                && requested_version.is_none_or(|version| entry.version.as_deref() == Some(version))
        })
        .ok_or_else(|| format!("content is not in the service Library: {selector}"))?;
    if entry.role != CatalogItemRole::PlayableComposition {
        return Err(format!(
            "content '{}' is {}, not a playable Packagepack",
            entry.id, entry.kind
        ));
    }
    let installed = entry
        .locations
        .iter()
        .find(|location| location.kind == CatalogLocationKind::Installed)
        .ok_or_else(|| format!("Packagepack '{}' is not installed", entry.id))?;
    if !installed.enabled {
        return Err(format!(
            "Packagepack '{}' is disabled; enable it before selecting it",
            entry.id
        ));
    }
    Ok((entry.id.clone(), entry.version.clone()))
}

fn print_catalog_item(entry: &CatalogEntry, location_kind: CatalogLocationKind) {
    println!("  {} ({})", entry.id, entry.kind);
    if let Some(name) = &entry.name {
        println!("    name: {name}");
    }
    if let Some(version) = &entry.version {
        println!("    version: {version}");
    }
    if let Some(workshop_id) = &entry.published_file_id {
        println!("    workshop: {workshop_id}");
    }
    for location in entry
        .locations
        .iter()
        .filter(|location| location.kind == location_kind)
    {
        println!("    location: {}", location.root);
        if location_kind == CatalogLocationKind::Installed {
            println!(
                "    state: {}",
                if location.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
    }
    for dependency in &entry.dependencies {
        println!("    dependency: {dependency}");
    }
    for conflict in &entry.conflicts {
        println!("    conflict: {}", conflict.field);
    }
}

#[cfg(test)]
mod workshop_intent_tests {
    use super::{staged_workshop_review_matches, validate_workshop_intent};

    #[test]
    fn workshop_create_and_update_keep_their_distinct_targets() {
        assert!(validate_workshop_intent("moon", None, "creation").is_ok());
        assert!(validate_workshop_intent("moon", Some("123"), "publication").is_ok());
        assert!(validate_workshop_intent("moon", Some("123"), "creation").is_err());
        assert!(validate_workshop_intent("moon", None, "publication").is_err());
    }

    #[test]
    fn staging_only_accepts_the_review_for_its_completed_operation() {
        assert!(!staged_workshop_review_matches(
            "old-operation",
            "example@0.1.0",
            "new-operation",
            "example@0.1.0"
        ));
        assert!(staged_workshop_review_matches(
            "old-operation",
            "example@0.1.0",
            "old-operation",
            "example@0.1.0"
        ));
        assert!(!staged_workshop_review_matches(
            "old-operation",
            "example@0.1.0",
            "old-operation",
            "other@0.1.0"
        ));
    }
}
