//! Playable composition lifecycle operations through the shared Client service.

use super::client_commands::{
    connect_client_service, print_service_operation_message, send_client_action,
};
use super::{CompositionCommand, ShellState};
use vapor_client_protocol::{ClientAction, CompositionAction};

pub(super) fn execute_composition(
    command: CompositionCommand,
    state: &ShellState,
) -> Result<(), String> {
    if matches!(command, CompositionCommand::List) {
        let client = connect_client_service(state)?;
        let snapshot = client.latest_snapshot();
        for composition in &snapshot.durable_state.compositions.compositions {
            let selected = snapshot
                .active_composition_id
                .as_deref()
                .unwrap_or(&snapshot.durable_state.active_composition_id)
                == composition.id;
            println!(
                "{}{}\t{}\t{}",
                if selected { "* " } else { "  " },
                composition.id,
                composition.name,
                installation_state_label(&composition.installation),
            );
        }
        return Ok(());
    }

    let operation = command.operation_name();
    let action = match command {
        CompositionCommand::List => unreachable!("list is handled before dispatch"),
        CompositionCommand::Install { composition_id } => {
            CompositionAction::Install { composition_id }
        }
        CompositionCommand::Verify { composition_id } => {
            CompositionAction::Verify { composition_id }
        }
        CompositionCommand::Repair { composition_id } => {
            CompositionAction::Repair { composition_id }
        }
        CompositionCommand::Update { composition_id } => {
            CompositionAction::Update { composition_id }
        }
        CompositionCommand::Launch { composition_id } => {
            CompositionAction::Launch { composition_id }
        }
    };
    let event = send_client_action(
        &mut connect_client_service(state)?,
        ClientAction::Composition(action),
    )?;
    print_service_operation_message(event);
    println!("composition {operation} completed through the shared Client service");
    Ok(())
}

fn installation_state_label(state: &vapor_client_protocol::InstallationState) -> &'static str {
    match state {
        vapor_client_protocol::InstallationState::Unknown => "not checked",
        vapor_client_protocol::InstallationState::NotInstalled => "not installed",
        vapor_client_protocol::InstallationState::Installed { .. } => "installed",
        vapor_client_protocol::InstallationState::OutOfDate { .. } => "update available",
        vapor_client_protocol::InstallationState::Partial { .. } => "partially installed",
        vapor_client_protocol::InstallationState::NeedsRepair { .. } => "needs repair",
    }
}
