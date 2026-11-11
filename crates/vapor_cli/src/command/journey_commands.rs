//! Shared guided onboarding progress commands.

use super::{JourneyCommand, ShellState, client_commands};
use vapor_client_application::{
    QuestAvailability, confirm_journey_quest, journey_progress, observe_developer_toolchain,
    observe_installed_composition,
};
use vapor_client_protocol::InstallationState;

pub(super) fn execute_journey(command: JourneyCommand, state: &ShellState) -> Result<(), String> {
    match command {
        JourneyCommand::Status => {
            let client = client_commands::connect_client_service(state)?;
            let service = client.latest_snapshot().clone();
            if let Some(composition) =
                service
                    .durable_state
                    .compositions
                    .compositions
                    .iter()
                    .find(|composition| {
                        matches!(
                            &composition.installation,
                            InstallationState::Installed { .. }
                        )
                    })
            {
                observe_installed_composition(&composition.id)?;
            }
            let role = service.installation_role;
            observe_developer_toolchain(
                &role.role,
                role.rust_ready,
                role.windows_gnu_target_ready,
                role.windows_gnu_linker_ready,
            )?;
            let progress = journey_progress()?;
            println!(
                "Journeys · {} points · {} of {} quests complete",
                progress.points, progress.completed_quests, progress.total_quests
            );
            for journey in progress.journeys {
                println!(
                    "\n{} — {}",
                    journey.definition.title, journey.definition.description
                );
                for quest in journey.quests {
                    let status = match &quest.availability {
                        QuestAvailability::Completed(_) => "done",
                        QuestAvailability::Available => "ready",
                        QuestAvailability::Locked(_) => "locked",
                    };
                    println!(
                        "  [{status}] {} (+{})  {}",
                        quest.definition.id, quest.definition.points, quest.definition.title
                    );
                    println!("         {}", quest.definition.description);
                    if let QuestAvailability::Locked(required) = quest.availability {
                        println!("         complete first: {}", required.join(", "));
                    }
                }
            }
        }
        JourneyCommand::Confirm { quest_id } => {
            let progress = confirm_journey_quest(&quest_id)?;
            println!(
                "Confirmed {quest_id}. Journey score: {} points, {} of {} quests.",
                progress.points, progress.completed_quests, progress.total_quests
            );
        }
    }
    Ok(())
}
