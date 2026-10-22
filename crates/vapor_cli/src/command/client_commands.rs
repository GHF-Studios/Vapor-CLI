//! CLI facade operations over the shared Vapor Client service.

use super::model::client_launch_arguments;
use super::{ClientCommand, ClientDeployCommand, ClientLaunchMode, LaunchCommand, ShellState};
use crate::{diagnostics, workflow};
use std::{
    io::Write,
    process::Command,
    time::{Duration, Instant},
};
use vapor_client::{Client, ClientHostConfig, service_sibling_path};
use vapor_client_protocol::{
    AuthoringResultKind, ClientAction, ClientEvent, ClientResponseKind, CompositionAction,
    OperationStatus, ProtocolFrame, WorkshopAction, WorkspaceAction,
};
use vapor_core::{LocalClientDeploymentOutcome, deploy_client_locally};

pub(crate) fn execute_client(command: ClientCommand) -> Result<(), String> {
    execute_client_body(command)
}

fn execute_client_body(command: ClientCommand) -> Result<(), String> {
    match command {
        ClientCommand::Launch { mode, arguments } => {
            return launch_client_entrypoint(mode, arguments);
        }
        ClientCommand::Deploy {
            command: ClientDeployCommand::Local,
        } => match deploy_client_locally().map_err(|error| error.to_string())? {
            LocalClientDeploymentOutcome::AuthorityDelegated {
                executable,
                installation_root,
            } => {
                println!("Transferring local deployment authority to source-built CLI:");
                println!("  executable: {}", executable.display());
                println!("  installation: {}", installation_root.display());
            }
            LocalClientDeploymentOutcome::Deployed {
                report,
                source_bootstrap,
                synchronization_warning,
            } => {
                println!("Vapor Installation: {}", report.installation_root.display());
                println!("Deployed binaries:");
                for binary in &report.binaries {
                    println!("  {} -> {}", binary.name, binary.destination.display());
                }
                println!("Launch wrapper: {}", report.launch_script.display());
                println!("Shell activation: {}", report.activation_script.display());
                println!("Source bootstrap: {}", source_bootstrap.display());
                if let Some(error) = synchronization_warning {
                    eprintln!(
                        "warning: local Client deployment succeeded, but the existing development environment could not be synchronized: {error}"
                    );
                }
            }
        },
    }
    Ok(())
}

pub(super) fn execute_launch(
    command: Option<LaunchCommand>,
    current_workspace: bool,
    profiling: bool,
    state: &ShellState,
) -> Result<(), String> {
    if current_workspace {
        if command.is_some() {
            return Err(
                "`launch --current-workspace` cannot be combined with an installed launch target"
                    .to_owned(),
            );
        }
        let workspace_root = std::env::current_dir()
            .map_err(|error| format!("could not read the current directory: {error}"))?;
        println!(
            "Launching the default Run Configuration from {} through the Vapor Client service…",
            workspace_root.display(),
        );
        let event = send_client_action(
            &mut connect_client_service(state)?,
            ClientAction::Workspace(WorkspaceAction::RunConfiguration {
                workspace_root: workspace_root.display().to_string(),
                configuration: "default".to_owned(),
                profiling,
            }),
        )?;
        print_service_operation_message(event);
        return Ok(());
    }

    if profiling {
        return Err("`--profiling` is supported with `launch --current-workspace`; installed game launches are not Vapor Development Sessions.".to_owned());
    }

    match command.unwrap_or(LaunchCommand::Selected) {
        // `default` remains accepted as a compatibility spelling, but it no
        // longer bypasses the service-owned persistent selection.
        LaunchCommand::Default | LaunchCommand::Selected => launch_selected_composition(state),
    }
}

fn launch_client_entrypoint(
    mode: ClientLaunchMode,
    arguments: Vec<std::ffi::OsString>,
) -> Result<(), String> {
    let installation = vapor_core::VaporInstallation::discover()
        .map_err(|error| format!("could not locate the Vapor installation: {error}"))?;
    let entrypoint = installation
        .root
        .join("bin")
        .join(workflow::host_runtime_target())
        .join(format!("vapor-entrypoint{}", std::env::consts::EXE_SUFFIX));
    if !entrypoint.is_file() {
        return Err(format!(
            "local Steam entrypoint was not found at {}. Run `vapor client deploy local` first.",
            entrypoint.display()
        ));
    }

    let status = Command::new(&entrypoint)
        .args(client_launch_arguments(mode, arguments))
        .current_dir(&installation.root)
        .status()
        .map_err(|error| format!("could not launch {}: {error}", entrypoint.display()))?;
    if !status.success() {
        return Err(format!("Vapor entrypoint exited with {status}"));
    }
    Ok(())
}

fn launch_selected_composition(state: &ShellState) -> Result<(), String> {
    let mut client = connect_client_service(state)?;
    let snapshot = client.latest_snapshot();
    let composition_id = snapshot
        .active_composition_id
        .as_deref()
        .unwrap_or(&snapshot.durable_state.active_composition_id)
        .to_owned();
    let composition_id = (!composition_id.is_empty()
        && snapshot
            .durable_state
            .compositions
            .compositions
            .iter()
            .any(|composition| composition.id == composition_id))
    .then_some(composition_id)
    .ok_or_else(|| {
        "no selected composition; open Studio and choose a playable composition first".to_owned()
    })?;
    launch_via_client_service(&mut client, &composition_id, "selected")
}

pub(crate) fn connect_client_service(state: &ShellState) -> Result<Client, String> {
    let installation = state.installation();
    let service_path = service_sibling_path(installation.executable());
    if !service_path.is_file() {
        return Err(format!(
            "Vapor Client service was not found at {}; repair or update the Vapor installation",
            service_path.display()
        ));
    }
    Client::connect_or_start(
        &service_path,
        ClientHostConfig {
            application_executable: installation.executable().to_path_buf(),
            workshop_app_id: None,
            workspace_root: None,
            active_composition_id: None,
            fixture_mode: false,
            legacy_storage: Default::default(),
            state_directory: None,
        },
    )
    .map_err(|error| error.to_string())
}

pub(super) fn active_source_root(state: &ShellState) -> Option<String> {
    state
        .active_paths()
        .ok()
        .map(|paths| paths.source().root().display().to_string())
}

pub(super) fn is_workshop_id(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn require_workshop_id(value: &str) -> Result<(), String> {
    if value
        .parse::<u64>()
        .is_ok_and(|published_file_id| published_file_id > 0)
    {
        Ok(())
    } else {
        Err("a Steam Workshop PublishedFileId must be a positive decimal number".to_owned())
    }
}

pub(super) fn acquire_workshop_items(
    state: &ShellState,
    item_ids: Vec<String>,
) -> Result<(), String> {
    for item_id in &item_ids {
        require_workshop_id(item_id)?;
    }
    let event = send_client_action(
        &mut connect_client_service(state)?,
        ClientAction::Workshop(WorkshopAction::Acquire { item_ids }),
    )?;
    println!("Workshop content is now available through the shared Client service.");
    print_service_operation_message(event);
    Ok(())
}

pub(super) fn print_service_operation_message(
    event: Option<vapor_client_protocol::OperationEvent>,
) {
    if let Some(event) = event {
        crate::console::print_operation_event(&event);
    }
}

pub(super) fn set_active_composition_root(
    state: &ShellState,
    root: Option<(&str, Option<&str>)>,
) -> Result<(), String> {
    let mut client = connect_client_service(state)?;
    let snapshot = client.latest_snapshot();
    let composition_id = snapshot
        .active_composition_id
        .as_deref()
        .unwrap_or(&snapshot.durable_state.active_composition_id)
        .to_owned();
    let composition = snapshot
        .durable_state
        .compositions
        .compositions
        .iter()
        .find(|composition| composition.id == composition_id)
        .ok_or_else(|| "the selected playable composition is unavailable".to_owned())?;
    let workspace_root = composition
        .workspace
        .as_ref()
        .map(|workspace| workspace.root.display().to_string());
    let next_content = root.map(|(id, _)| id.to_owned());
    let next_version = root.and_then(|(_, version)| version.map(str::to_owned));
    if composition.root_package.as_ref().is_some_and(|current| {
        Some(current.id.as_str()) == next_content.as_deref()
            && current.version.as_deref() == next_version.as_deref()
    }) || (composition.root_package.is_none() && next_content.is_none())
    {
        return Ok(());
    }
    send_client_action(
        &mut client,
        ClientAction::Composition(CompositionAction::SetRoot {
            composition_id,
            content_id: next_content,
            content_version: next_version,
            workspace_root,
        }),
    )?;
    Ok(())
}

fn launch_via_client_service(
    client: &mut Client,
    composition_id: &str,
    request: &str,
) -> Result<(), String> {
    let composition = client
        .latest_snapshot()
        .durable_state
        .compositions
        .compositions
        .iter()
        .find(|composition| composition.id == composition_id)
        .ok_or_else(|| format!("composition '{composition_id}' is no longer available"))?;
    if composition.root_package.is_none() {
        return Err(format!(
            "composition '{}' has no Packagepack; choose one in Studio before launching",
            composition.name
        ));
    }
    println!("Launching {} composition…", composition.name);
    diagnostics::record_launch_target(request);
    diagnostics::event(format!(
        "launch {request} started through the shared Client service; composition={composition_id}"
    ));
    let event = send_client_action(
        client,
        ClientAction::Composition(CompositionAction::Launch {
            composition_id: composition_id.to_owned(),
        }),
    )?;
    if let Some(message) = event.and_then(|event| event.message)
        && !message.trim().is_empty()
    {
        println!("{message}");
    }
    diagnostics::event(format!(
        "launch {request} completed through the shared Client service; composition={composition_id}"
    ));
    Ok(())
}

/// Dispatches one service action and waits until any accepted long-running
/// operation reaches a terminal state. The operation outcome, not acceptance,
/// determines the CLI result.
pub(crate) fn send_client_action(
    client: &mut Client,
    action: ClientAction,
) -> Result<Option<vapor_client_protocol::OperationEvent>, String> {
    send_client_action_inner(client, action, false).map(|(event, _)| event)
}

pub(crate) fn send_client_action_with_authoring_result(
    client: &mut Client,
    action: ClientAction,
) -> Result<
    (
        Option<vapor_client_protocol::OperationEvent>,
        Option<AuthoringResultKind>,
    ),
    String,
> {
    send_client_action_inner(client, action, true)
}

fn send_client_action_inner(
    client: &mut Client,
    action: ClientAction,
    capture_authoring_result: bool,
) -> Result<
    (
        Option<vapor_client_protocol::OperationEvent>,
        Option<AuthoringResultKind>,
    ),
    String,
> {
    const RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
    const OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);
    const ACTION_RESULT_TIMEOUT: Duration = Duration::from_secs(5);

    let expects_authoring_result =
        capture_authoring_result && matches!(&action, ClientAction::Authoring(_));
    let request_id = client
        .send(action, Some(client.latest_snapshot().revision))
        .map_err(|error| error.to_string())?;
    diagnostics::event(format!(
        "service request {request_id:?} sent; authoring result expected={expects_authoring_result}"
    ));
    let deadline = Instant::now() + RESPONSE_TIMEOUT;
    let response = loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("timed out waiting for Vapor Client service response".to_owned());
        }
        match client.recv_timeout(remaining).map_err(|error| {
            diagnostics::event(format!(
                "service request {request_id:?} failed while awaiting response: {error}"
            ));
            error.to_string()
        })? {
            ProtocolFrame::Response(response) if response.request_id == request_id => {
                diagnostics::event(format!(
                    "service request {request_id:?} returned {:?}; request operation={}",
                    response.kind,
                    response.operation_id.as_deref().unwrap_or("none")
                ));
                break *response;
            }
            ProtocolFrame::Event(ClientEvent::ProtocolError(error)) => return Err(error.message),
            ProtocolFrame::Response(_) => continue,
            ProtocolFrame::Event(_) => continue,
            _ => return Err("Vapor Client service sent an unexpected protocol frame".to_owned()),
        }
    };
    if response.kind == ClientResponseKind::Rejected {
        return Err(response
            .error
            .map(|error| error.message)
            .unwrap_or_else(|| "Vapor Client service rejected the request".to_owned()));
    }
    if response.kind != ClientResponseKind::OperationAccepted {
        return Ok((None, None));
    }
    let operation_id = response
        .operation_id
        .clone()
        .ok_or_else(|| "service accepted an operation without returning its identity".to_owned())?;
    let deadline = Instant::now() + OPERATION_TIMEOUT;
    let mut completed_event = None;
    let mut authoring_result = None;
    let mut action_result_deadline: Option<Instant> = None;
    let mut last_rendered_progress = None;
    let mut last_logged_state = None;
    loop {
        let now = Instant::now();
        let remaining = action_result_deadline.map_or_else(
            || deadline.saturating_duration_since(now),
            |result_deadline| result_deadline.saturating_duration_since(now),
        );
        if remaining.is_zero() {
            if completed_event.is_some() {
                return Err(format!(
                    "Vapor Client completed operation {operation_id} without returning its authoring report"
                ));
            }
            return Err(format!(
                "timed out waiting for service operation {operation_id}"
            ));
        }
        match client
            .recv_timeout(remaining)
            .map_err(|error| error.to_string())?
        {
            ProtocolFrame::Event(ClientEvent::OperationChanged(event))
                if event.id == operation_id =>
            {
                let state = (event.phase, event.status.clone());
                if last_logged_state.as_ref() != Some(&state) {
                    let status = match &event.status {
                        OperationStatus::Running => "running",
                        OperationStatus::Cancelling => "cancelling",
                        OperationStatus::Succeeded => "succeeded",
                        OperationStatus::Failed { .. } => "failed",
                        OperationStatus::Cancelled => "cancelled",
                    };
                    diagnostics::event(format!(
                        "service operation {} {} {:?} {status}",
                        event.id, event.action, event.phase
                    ));
                    last_logged_state = Some(state);
                }
                match event.status.clone() {
                    OperationStatus::Succeeded if expects_authoring_result => {
                        finish_cli_progress(last_rendered_progress.is_some());
                        if let Some(result) = authoring_result.take() {
                            return Ok((Some(event), Some(result)));
                        }
                        completed_event = Some(event);
                        action_result_deadline = Some(Instant::now() + ACTION_RESULT_TIMEOUT);
                    }
                    OperationStatus::Succeeded => {
                        finish_cli_progress(last_rendered_progress.is_some());
                        return Ok((Some(event), None));
                    }
                    OperationStatus::Failed { message } => {
                        finish_cli_progress(last_rendered_progress.is_some());
                        return Err(message);
                    }
                    OperationStatus::Cancelled => {
                        finish_cli_progress(last_rendered_progress.is_some());
                        return Err("the launch operation was cancelled".to_owned());
                    }
                    OperationStatus::Cancelling => {
                        finish_cli_progress(last_rendered_progress.is_some());
                        last_rendered_progress = None;
                        eprintln!("{}: stopping…", event.action);
                    }
                    OperationStatus::Running => {
                        if let Some(progress) = event.progress {
                            let percent = (progress.clamp(0.0, 1.0) * 100.0).round() as u8;
                            let rendered = (percent, event.message.clone());
                            if last_rendered_progress != Some(rendered.clone()) {
                                if let Some(message) = event.message.as_deref() {
                                    eprint!("\r{}: {percent}% — {message}", event.action);
                                } else {
                                    eprint!("\r{}: {percent}%", event.action);
                                }
                                let _ = std::io::stderr().flush();
                                last_rendered_progress = Some(rendered);
                            }
                        }
                    }
                }
            }
            ProtocolFrame::Event(ClientEvent::OperationOutput(output))
                if output.id == operation_id =>
            {
                finish_cli_progress(last_rendered_progress.is_some());
                last_rendered_progress = None;
                let write_result = match output.stream {
                    vapor_client_protocol::OperationOutputStream::Stdout => {
                        let mut stdout = std::io::stdout().lock();
                        writeln!(stdout, "{}", output.text).and_then(|_| stdout.flush())
                    }
                    vapor_client_protocol::OperationOutputStream::Stderr => {
                        let mut stderr = std::io::stderr().lock();
                        writeln!(stderr, "{}", output.text).and_then(|_| stderr.flush())
                    }
                };
                write_result.map_err(|error| {
                    format!("could not render managed operation output: {error}")
                })?;
            }
            ProtocolFrame::Event(ClientEvent::AuthoringResult(result))
                if expects_authoring_result && result.operation_id == operation_id =>
            {
                if let Some(event) = completed_event.take() {
                    return Ok((Some(event), Some(result.result)));
                }
                authoring_result = Some(result.result);
            }
            ProtocolFrame::Event(ClientEvent::ProtocolError(error)) => return Err(error.message),
            ProtocolFrame::Event(_) | ProtocolFrame::Response(_) => {}
            _ => return Err("Vapor Client service sent an unexpected protocol frame".to_owned()),
        }
    }
}

fn finish_cli_progress(was_rendered: bool) {
    if was_rendered {
        eprintln!();
    }
}
