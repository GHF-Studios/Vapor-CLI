//! Execution of authored-source registration and workspace commands.

use super::{ShellState, SourceCommand, SourceTemplate, print_warnings};
use crate::{diagnostics, discovery::EnvironmentPaths, workspace::WorkspaceManifest};
use std::path::PathBuf;
use vapor_client_protocol::{AuthoringAction, AuthoringResultKind, ClientAction};

pub(super) fn execute_source(command: SourceCommand, state: &mut ShellState) -> Result<(), String> {
    match command {
        SourceCommand::Init {
            template,
            path,
            organization,
            name,
            app_id,
        } => execute_source_init(template, path, organization, name, app_id, state),
        SourceCommand::Status => execute_source_status(state),
        SourceCommand::Open { source } => execute_source_open(&source, state),
        SourceCommand::Close => execute_source_close(state),
        SourceCommand::List => execute_source_list(state),
        SourceCommand::Add { path } => execute_source_add(state, path),
        SourceCommand::Remove { source } => execute_source_remove(state, &source),
        SourceCommand::Sync => {
            let source = state.active_paths()?.source();
            println!("source: {}", source.identity_id());
            println!("  root: {}", source.root().display());
            println!(
                "sync: not yet applied; controlled Git/provider synchronization will attach here"
            );
            println!("hint: inspect source state with `source status`");
            Ok(())
        }
        SourceCommand::Repair { write } => {
            let sources = registered_sources(state)?;
            let metadata_repair = if let Ok(paths) = state.active_paths() {
                let mut client = super::connect_client_service(state)?;
                let (_, result) = super::send_client_action_with_authoring_result(
                    &mut client,
                    ClientAction::Authoring(AuthoringAction::RepairSourceMetadata {
                        workspace_root: paths.source().root().display().to_string(),
                        write_changes: write,
                    }),
                )?;
                match result {
                    Some(AuthoringResultKind::SourceMetadataRepaired {
                        write_changes,
                        actions,
                    }) => Some((write_changes, actions)),
                    _ => {
                        return Err(
                            "Vapor Client service returned an unexpected source repair result"
                                .to_owned(),
                        );
                    }
                }
            } else {
                None
            };
            println!("Source Repair");
            println!();
            println!("Status");
            println!("  Saved sources: {}", sources.len());
            if let Some(source) = state.source() {
                println!("  Source project: {}", source.id());
                println!("    root: {}", source.root().display());
                match WorkspaceManifest::load(state.active_paths()?) {
                    Ok(manifest) => {
                        println!("  Registered Vapor projects: {}", manifest.projects().len());
                        print_registered_projects(&manifest);
                    }
                    Err(error) => {
                        println!("  Workspace manifest: invalid");
                        println!("    {error}");
                    }
                }
                if let Some((write_changes, actions)) = &metadata_repair {
                    if actions.is_empty() {
                        println!("  Metadata repairs: none");
                    } else {
                        println!(
                            "  Metadata repairs: {} {}",
                            actions.len(),
                            if *write_changes {
                                "applied"
                            } else {
                                "available"
                            }
                        );
                        for action in actions {
                            println!(
                                "    - [{}] {} -> workshop-id {}",
                                action.section, action.reference_id, action.workshop_id
                            );
                            println!("      manifest: {}", action.manifest);
                        }
                    }
                }
            } else {
                println!("  Source project: none open");
            }
            println!();
            println!("Next");
            if state.source().is_some() {
                if metadata_repair
                    .as_ref()
                    .is_some_and(|(write_changes, actions)| !*write_changes && !actions.is_empty())
                {
                    println!("  source repair --write");
                } else {
                    println!("  source status");
                }
            } else if sources.is_empty() {
                println!("  source open /path/to/source");
            } else {
                println!("  source list");
            }
            Ok(())
        }
    }
}

fn execute_source_init(
    template: SourceTemplate,
    path: PathBuf,
    organization: String,
    name: String,
    app_id: Option<u32>,
    state: &mut ShellState,
) -> Result<(), String> {
    let target = if path.is_absolute() {
        path
    } else if let Ok(current) = state.current_dir() {
        current.join(path)
    } else {
        std::env::current_dir()
            .map_err(|error| format!("failed to read invocation directory: {error}"))?
            .join(path)
    };
    let (root, workspace_id, packagepack_id) = match template {
        SourceTemplate::BasicContent => {
            let mut client = super::connect_client_service(state)?;
            let (_, result) = super::send_client_action_with_authoring_result(
                &mut client,
                ClientAction::Authoring(AuthoringAction::InitializeSourceWorkspace {
                    path: target.display().to_string(),
                    organization,
                    name,
                    app_id,
                }),
            )?;
            match result {
                Some(AuthoringResultKind::SourceWorkspaceCreated {
                    root,
                    workspace_id,
                    packagepack_id,
                    ..
                }) => (PathBuf::from(root), workspace_id, packagepack_id),
                _ => {
                    return Err(
                        "Vapor Client service returned an unexpected workspace result".to_owned(),
                    );
                }
            }
        }
    };
    let paths =
        EnvironmentPaths::from_installation_and_source_path(state.installation().clone(), &root)?;
    let name = source_name(paths.source().identity_id()).to_owned();
    let id = paths.source().identity_id().to_owned();
    let root = paths.source().root().to_path_buf();
    print_warnings(state.open_paths(paths)?);
    diagnostics::record_source_context(state.source());

    println!("Source Created");
    println!();
    println!("Status");
    println!("  Template: basic-content");
    println!("  Source project: {name}");
    println!("    id: {id}");
    println!("    root: {}", root.display());
    println!("  Workspace: {workspace_id}");
    println!("  Packagepack: {packagepack_id}");
    println!();
    println!("Next");
    println!("  content validate");
    println!();
    println!("Then");
    println!("  content deploy {packagepack_id} --select");
    Ok(())
}

fn execute_source_status(state: &ShellState) -> Result<(), String> {
    let sources = registered_sources(state)?;
    println!("Source Status");
    println!();
    println!("Status");
    if let Some(source) = state.source() {
        println!("  Source project: open");
        println!("    id: {}", source.id());
        println!("    root: {}", source.root().display());
        println!("    current: {}", state.current_dir()?.display());
        match WorkspaceManifest::load(state.active_paths()?) {
            Ok(manifest) => {
                println!("  Cargo workspaces: {}", manifest.cargo_projects().len());
                println!("  Registered Vapor projects: {}", manifest.projects().len());
                print_registered_projects(&manifest);
            }
            Err(error) => {
                println!("  Workspace manifest: invalid");
                println!("    {error}");
            }
        }
    } else {
        println!("  Source project: none open");
    }
    println!("  Saved sources: {}", sources.len());
    println!();
    println!("Next");
    if state.source().is_some() {
        println!("  metadata");
    } else if sources.is_empty() {
        println!("  source open /path/to/source");
    } else {
        println!("  source list");
    }
    Ok(())
}

fn print_registered_projects(manifest: &WorkspaceManifest) {
    for project in manifest.projects() {
        println!("    - {} ({})", project.id(), project.kind());
        println!("      path: {}", project.path().display());
    }
}

fn execute_source_open(source: &str, state: &mut ShellState) -> Result<(), String> {
    let path = resolve_source_target(state, source)?;
    let paths =
        EnvironmentPaths::from_installation_and_source_path(state.installation().clone(), &path)?;
    apply_source_action(
        state,
        AuthoringAction::OpenSource {
            root: paths.source().root().display().to_string(),
        },
    )?;
    let name = source_name(paths.source().identity_id()).to_owned();
    let id = paths.source().identity_id().to_owned();
    let root = paths.source().root().to_path_buf();
    print_warnings(state.open_paths(paths)?);
    diagnostics::record_source_context(state.source());
    println!("Source Opened");
    println!();
    println!("Status");
    println!("  Source project: {name}");
    println!("    id: {id}");
    println!("    root: {}", root.display());
    println!();
    println!("Next");
    println!("  metadata");
    println!();
    println!("Then");
    println!("  validate");
    Ok(())
}

fn execute_source_close(state: &mut ShellState) -> Result<(), String> {
    apply_source_action(state, AuthoringAction::CloseSource)?;
    state.close_source();
    diagnostics::record_source_context(state.source());
    println!("Source Closed");
    println!();
    println!("Next");
    println!("  source list");
    Ok(())
}

fn execute_source_list(state: &ShellState) -> Result<(), String> {
    let sources = registered_sources(state)?;
    println!("Saved Sources");
    println!();
    if sources.is_empty() {
        println!("Status");
        println!("  Saved sources: none");
        println!();
        println!("Next");
        println!("  source open /path/to/source");
    } else {
        println!("Status");
        for root in sources {
            let identity = EnvironmentPaths::from_installation_and_source_path(
                state.installation().clone(),
                std::path::Path::new(&root),
            )
            .map(|paths| paths.source().identity_id().to_owned())
            .unwrap_or_else(|_| "unavailable source".to_owned());
            println!("  {}", source_name(&identity));
            println!("    id: {identity}");
            println!("    root: {root}");
        }
        println!();
        println!("Next");
        println!("  source open NAME");
    }
    Ok(())
}

fn execute_source_add(state: &ShellState, path: Option<PathBuf>) -> Result<(), String> {
    let path = path.unwrap_or(
        std::env::current_dir()
            .map_err(|error| format!("failed to read the invocation directory: {error}"))?,
    );
    let paths =
        EnvironmentPaths::from_installation_and_source_path(state.installation().clone(), &path)?;
    let name = source_name(paths.source().identity_id());
    let id = paths.source().identity_id().to_owned();
    let root = paths.source().root().to_path_buf();
    apply_source_action(
        state,
        AuthoringAction::RememberSource {
            root: root.display().to_string(),
        },
    )?;
    println!("Source Saved");
    println!();
    println!("Status");
    println!("  Source project: {name}");
    println!("    id: {id}");
    println!("    root: {}", root.display());
    println!();
    println!("Next");
    println!("  source open {name}");
    Ok(())
}

fn execute_source_remove(state: &ShellState, source: &str) -> Result<(), String> {
    let root = resolve_source_target(state, source)?;
    apply_source_action(
        state,
        AuthoringAction::ForgetSource {
            root: root.display().to_string(),
        },
    )?;
    println!("Forgot source: {}", root.display());
    Ok(())
}

fn apply_source_action(state: &ShellState, action: AuthoringAction) -> Result<(), String> {
    let mut client = super::connect_client_service(state)?;
    let (_, result) = super::send_client_action_with_authoring_result(
        &mut client,
        ClientAction::Authoring(action),
    )?;
    if !matches!(result, Some(AuthoringResultKind::RegisteredSources { .. })) {
        return Err("Vapor Client service returned an unexpected source result".to_owned());
    }
    Ok(())
}

fn registered_sources(state: &ShellState) -> Result<Vec<String>, String> {
    let mut client = super::connect_client_service(state)?;
    let (_, result) = super::send_client_action_with_authoring_result(
        &mut client,
        ClientAction::Authoring(AuthoringAction::ListRegisteredSources),
    )?;
    match result {
        Some(AuthoringResultKind::RegisteredSources { roots, .. }) => Ok(roots),
        _ => Err("Vapor Client service did not return registered sources".to_owned()),
    }
}

fn resolve_source_target(state: &ShellState, source: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(source);
    if candidate.exists() || source.contains('/') || source.contains('\\') || source == "." {
        return Ok(candidate);
    }
    for root in registered_sources(state)? {
        let Ok(paths) = EnvironmentPaths::from_installation_and_source_path(
            state.installation().clone(),
            std::path::Path::new(&root),
        ) else {
            continue;
        };
        let identity = paths.source().identity_id();
        if identity == source || source_name(identity) == source {
            return Ok(PathBuf::from(root));
        }
    }
    Err(format!(
        "unknown Vapor source '{source}'\nhelp: add it with `vapor source add PATH`, or open by path"
    ))
}

fn source_name(identity: &str) -> &str {
    identity.rsplit('/').next().unwrap_or(identity)
}

pub(crate) fn active_source_root(state: &ShellState) -> Result<Option<PathBuf>, String> {
    if let Some(root) = std::env::var_os("VAPOR_WORKSPACE").filter(|value| !value.is_empty()) {
        return Ok(Some(PathBuf::from(root)));
    }
    let mut client = super::connect_client_service(state)?;
    crate::diagnostics::event(
        "source context: requesting registered-source selection from service",
    );
    let (_, result) = super::send_client_action_with_authoring_result(
        &mut client,
        ClientAction::Authoring(AuthoringAction::ListRegisteredSources),
    )?;
    match result {
        Some(AuthoringResultKind::RegisteredSources { active, .. }) => {
            crate::diagnostics::event(format!(
                "source context: service returned active source: {}",
                active.as_deref().unwrap_or("none")
            ));
            Ok(active.map(PathBuf::from))
        }
        _ => Err("Vapor Client service did not return the active source".to_owned()),
    }
}
