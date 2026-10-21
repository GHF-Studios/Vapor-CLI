//! Interactive command grammar and execution.
//!
//! Static finite argument domains use Clap enums. Dynamic domains, such as
//! discovered Cargo workspace names, are validated by Vapor metadata.
//!
//! Installation commands report app-root resource paths; they never move
//! the source cursor into the Steam application directory.

use crate::{
    app_local_tools::AppToolRequirement,
    content,
    diagnostics::{self, UploadOptions},
    discovery::EnvironmentPaths,
    documentation, git_provider, ide,
    metadata::{ResolvedMetadata, ValidationPlan},
    state::ShellState,
    workflow::{CargoWorkflow, ProjectSelection, host_runtime_target},
    workspace::{SourceRootKind, WorkspaceManifest},
};
mod client_commands;
mod composition_commands;
pub(crate) use client_commands::execute_client;
pub(crate) use client_commands::{
    connect_client_service, send_client_action, send_client_action_with_authoring_result,
};
pub(crate) use source_commands::active_source_root;
mod content_commands;
mod model;
mod root_commands;
mod source_commands;
pub use model::*;

use clap::Parser;

use std::{collections::BTreeSet, fs, path::PathBuf};
use vapor_client_protocol::{
    ClientAction, WorkspaceAction, WorkspaceProjectSelection, WorkspaceToolOperation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Whether the application loop should read another command.
pub enum Control {
    /// Read another command.
    Continue,
    /// End the interactive session.
    Exit,
}

/// Execute one parsed command against session state.
///
/// # Errors
///
/// Returns navigation, filesystem, or missing-installation-resource errors.
pub fn execute(command: ShellCommand, state: &mut ShellState) -> Result<Control, String> {
    execute_body(command, state)
}

fn execute_body(command: ShellCommand, state: &mut ShellState) -> Result<Control, String> {
    match command {
        ShellCommand::Launch {
            current_workspace,
            profiling,
            command,
        } => {
            client_commands::execute_launch(command, current_workspace, profiling, state)?;
        }
        ShellCommand::Composition { command } => {
            composition_commands::execute_composition(command, state)?;
        }
        ShellCommand::Toolchain { command } => match command {
            ToolchainCommand::Tracy => {
                let mut client = client_commands::connect_client_service(state)?;
                client_commands::send_client_action(
                    &mut client,
                    vapor_client_protocol::ClientAction::Developer(
                        vapor_client_protocol::DeveloperAction::OpenProfiler,
                    ),
                )?;
            }
        },
        ShellCommand::Client { command } => client_commands::execute_client(command)?,
        ShellCommand::Source { command } => source_commands::execute_source(command, state)?,
        ShellCommand::Installation => {
            println!("{}", state.installation().root().display());
        }
        ShellCommand::Binaries => {
            println!("{}", state.installation().binaries().display());
        }
        ShellCommand::Libraries => {
            let target = state.installation().libraries().ok_or_else(|| {
                format!(
                    "Steam installation '{}' has no library directory",
                    state.installation().root().display()
                )
            })?;
            println!("{}", target.display());
        }
        ShellCommand::Metadata { format } => {
            println!("{}", ResolvedMetadata::resolve(state).render(format)?);
        }
        ShellCommand::Fmt { project } => execute_workflow(CargoWorkflow::Fmt, project, state)?,
        ShellCommand::Check { project } => {
            execute_workflow(CargoWorkflow::Check, project, state)?;
        }
        ShellCommand::Test { project } => execute_workflow(CargoWorkflow::Test, project, state)?,
        ShellCommand::Build { project } => {
            execute_workflow(CargoWorkflow::Build, project, state)?;
        }
        ShellCommand::Clean => execute_clean_workspace(state)?,
        ShellCommand::Validate { project } => {
            execute_workflow(CargoWorkflow::Validate, project, state)?;
        }
        ShellCommand::Docs { command } => execute_docs(command, state)?,
        ShellCommand::Ide { command } => execute_ide(command, state)?,
        ShellCommand::Root { command } => root_commands::execute_root(command, state)?,
        ShellCommand::Content { command } => content_commands::execute_content(command, state)?,
        ShellCommand::Script { command } => execute_script_command(command, state)?,
        ShellCommand::Provider { command } => execute_provider(command, state)?,
        ShellCommand::Sdk { args } => {
            let arguments = std::iter::once(std::ffi::OsString::from("vapor sdk")).chain(args);
            vapor_sdk_cli::run(arguments)?;
        }
        ShellCommand::Diagnostics { command } => execute_diagnostics(command, state)?,
        ShellCommand::Exit => return Ok(Control::Exit),
    }

    Ok(Control::Continue)
}

fn execute_clean_workspace(state: &ShellState) -> Result<(), String> {
    let root = state.active_paths()?.source().root().to_path_buf();
    let mut client = client_commands::connect_client_service(state)?;
    content_commands::open_workspace_in_service(&mut client, state)?;
    let event = client_commands::send_client_action(
        &mut client,
        ClientAction::Workspace(WorkspaceAction::Clean {
            confirmed_workspace_root: root.display().to_string(),
        }),
    )?;
    client_commands::print_service_operation_message(event);
    Ok(())
}

fn execute_ide(command: IdeCommand, state: &ShellState) -> Result<(), String> {
    let metadata = ResolvedMetadata::resolve(state);
    match command {
        IdeCommand::Status => {
            metadata.validate(&ValidationPlan::new("inspect IDE setup").workspace())?;
            let status = ide::inspect(
                state.active_paths()?,
                metadata.workspace_manifest()?,
                metadata.app_local_tools_status(),
            )?;
            print_ide_status(&status);
            if status.complete() {
                println!("hint: IDE settings are current; next run `validate`");
            } else {
                println!("hint: preview project-local IDE repair with `ide repair --dry-run`");
            }
        }
        IdeCommand::Repair { dry_run } => {
            metadata.validate(
                &ValidationPlan::new("repair IDE setup")
                    .app_local_tools(&[AppToolRequirement::Rust])
                    .workspace(),
            )?;
            let report = if dry_run {
                ide::preview(
                    state.active_paths()?,
                    metadata.workspace_manifest()?,
                    metadata.app_local_tools_status(),
                )?
            } else {
                ide::repair(
                    state.active_paths()?,
                    metadata.workspace_manifest()?,
                    metadata.app_local_tools_status(),
                )?
            };
            print_ide_status(report.status());
            if report.written().is_empty() {
                println!(
                    "{}: IDE settings are already current",
                    if dry_run { "dry-run" } else { "repair" }
                );
            } else {
                for path in report.written() {
                    println!(
                        "{}: {}",
                        if dry_run { "would write" } else { "wrote" },
                        path.display()
                    );
                }
            }
            if dry_run {
                println!("dry-run: no IDE files were changed");
                println!("hint: apply with `ide repair`");
            } else {
                println!("hint: restart or refresh RustRover so project settings are reloaded");
            }
        }
    }
    Ok(())
}

fn print_ide_status(status: &ide::IdeStatus) {
    println!("IDE project root: {}", status.project_root().display());
    println!("source root: {}", status.source_root().display());
    println!("IDE directory: {}", status.idea_dir().display());
    println!("Rust/Cargo bin: {}", status.rust_bin().display());
    match status.stdlib_source() {
        Some(path) => println!("Rust standard library source: {}", path.display()),
        None => println!(
            "Rust standard library source: missing\nhint: include rust-src in the app-local Rust package for full IDE indexing"
        ),
    }
    for file in status.files() {
        let state = match file.state() {
            ide::IdeFileState::Missing => "missing",
            ide::IdeFileState::Outdated => "outdated",
            ide::IdeFileState::Current => "current",
        };
        println!("{state}: {}", file.path().display());
    }
}

fn execute_workflow(
    command: CargoWorkflow,
    project: ProjectSelection,
    state: &ShellState,
) -> Result<(), String> {
    let metadata = ResolvedMetadata::resolve(state);
    let paths = state.active_paths()?;
    let manifest = metadata.workspace_manifest()?;
    execute_workspace_tool(state, command, project.into(), &[])?;
    print_workflow_complete(command, paths, manifest);
    Ok(())
}

pub(super) fn execute_workspace_tool(
    state: &ShellState,
    workflow: CargoWorkflow,
    project_selection: WorkspaceProjectSelection,
    target_triples: &[String],
) -> Result<Option<vapor_client_protocol::OperationEvent>, String> {
    let operation = match workflow {
        CargoWorkflow::Fmt => WorkspaceToolOperation::Format,
        CargoWorkflow::Check => WorkspaceToolOperation::Check,
        CargoWorkflow::Test => WorkspaceToolOperation::Test,
        CargoWorkflow::Build => WorkspaceToolOperation::Build,
        CargoWorkflow::BuildRelease => WorkspaceToolOperation::BuildRelease,
        CargoWorkflow::Validate => WorkspaceToolOperation::Validate,
    };
    let mut client = connect_client_service(state)?;
    content_commands::open_workspace_in_service(&mut client, state)?;
    let event = send_client_action(
        &mut client,
        ClientAction::Workspace(WorkspaceAction::RunTool {
            operation,
            project_selection,
            target_triples: target_triples.to_vec(),
        }),
    )?;
    client_commands::print_service_operation_message(event.clone());
    Ok(event)
}

impl From<ProjectSelection> for WorkspaceProjectSelection {
    fn from(value: ProjectSelection) -> Self {
        match value {
            ProjectSelection::All => Self::All,
            ProjectSelection::Installable => Self::Installable,
            ProjectSelection::One(name) => Self::Named(name),
        }
    }
}

fn print_workflow_complete(
    command: CargoWorkflow,
    paths: &EnvironmentPaths,
    manifest: &WorkspaceManifest,
) {
    println!();
    println!("Workflow Complete");
    println!();
    println!("Status");
    println!("  {}: passed", command.label());
    println!("  Source type: {}", source_type_label(manifest, paths));
    println!();
    println!("Next");
    println!("  {}", workflow_next_command(command, paths, manifest));
}

fn source_type_label(manifest: &WorkspaceManifest, paths: &EnvironmentPaths) -> &'static str {
    match manifest.kind() {
        SourceRootKind::Root => "application root",
        SourceRootKind::Workspace if workspace_has_content(paths) => "content workspace",
        SourceRootKind::Workspace => "workspace",
    }
}

fn workflow_next_command(
    command: CargoWorkflow,
    paths: &EnvironmentPaths,
    manifest: &WorkspaceManifest,
) -> &'static str {
    match (manifest.kind(), workspace_has_content(paths), command) {
        (_, _, CargoWorkflow::Fmt) => "test",
        (_, _, CargoWorkflow::Check) => "test",
        (_, _, CargoWorkflow::Test) => "validate",
        (
            SourceRootKind::Root,
            _,
            CargoWorkflow::Build | CargoWorkflow::BuildRelease | CargoWorkflow::Validate,
        ) => "root build",
        (SourceRootKind::Workspace, true, CargoWorkflow::Build) => "content package ARTIFACT",
        (SourceRootKind::Workspace, true, CargoWorkflow::BuildRelease) => {
            "content package ARTIFACT"
        }
        (SourceRootKind::Workspace, true, CargoWorkflow::Validate) => "content validate",
        (SourceRootKind::Workspace, false, CargoWorkflow::Build) => "validate",
        (SourceRootKind::Workspace, false, CargoWorkflow::BuildRelease) => "validate",
        (SourceRootKind::Workspace, false, CargoWorkflow::Validate) => "build",
    }
}

fn workspace_has_content(paths: &EnvironmentPaths) -> bool {
    content::discover(paths)
        .map(|catalog| !catalog.artifacts().is_empty())
        .unwrap_or(false)
}

fn resolve_runtime_targets(
    manifest: &WorkspaceManifest,
    explicit_targets: &[String],
    release_targets: bool,
    host_only: bool,
) -> Result<Vec<String>, String> {
    let selected_modes = usize::from(!explicit_targets.is_empty())
        + usize::from(release_targets)
        + usize::from(host_only);
    if selected_modes > 1 {
        return Err("use only one of --target, --release-targets, or --host-only".to_owned());
    }
    if !explicit_targets.is_empty() {
        validate_command_runtime_targets(explicit_targets)?;
        return Ok(explicit_targets.to_vec());
    }
    if host_only {
        return Ok(vec![host_runtime_target()]);
    }
    if release_targets && manifest.runtime_targets().is_empty() {
        return Err(
            "source manifest declares no runtime targets; add [root.runtime]/[workspace.runtime] targets or pass --target"
                .to_owned(),
        );
    }
    if !manifest.runtime_targets().is_empty() {
        return Ok(manifest.runtime_targets().to_vec());
    }
    Ok(Vec::new())
}

fn resolve_publish_runtime_targets(
    manifest: &WorkspaceManifest,
    explicit_targets: &[String],
    release_targets: bool,
    host_only: bool,
    dry_run: bool,
    command: &str,
) -> Result<Vec<String>, String> {
    if dry_run {
        return resolve_runtime_targets(manifest, explicit_targets, release_targets, host_only);
    }
    if !explicit_targets.is_empty() {
        return Err(format!(
            "real {command} requires the complete declared runtime target matrix; remove --target or use --dry-run for custom target previews"
        ));
    }
    if host_only {
        return Err(format!(
            "real {command} requires the complete declared runtime target matrix; --host-only is only for dry-run/local smoke paths"
        ));
    }
    let targets = resolve_runtime_targets(manifest, &[], release_targets, false)?;
    require_release_runtime_matrix(command, &targets)?;
    Ok(targets)
}

fn require_release_runtime_matrix(command: &str, targets: &[String]) -> Result<(), String> {
    let has_linux = targets.iter().any(|target| target.contains("linux"));
    let has_windows = targets.iter().any(|target| target.contains("windows"));
    if has_linux && has_windows {
        return Ok(());
    }
    let declared = if targets.is_empty() {
        "<none>".to_owned()
    } else {
        targets.join(", ")
    };
    Err(format!(
        "real {command} requires a declared release runtime matrix containing both Linux and Windows targets; current targets: {declared}"
    ))
}

fn validate_command_runtime_targets(targets: &[String]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for target in targets {
        let valid = !target.is_empty()
            && target.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
            });
        if !valid {
            return Err(format!(
                "runtime target must be a Rust target triple such as x86_64-pc-windows-gnullvm: {target}"
            ));
        }
        if !seen.insert(target) {
            return Err(format!("duplicate runtime target: {target}"));
        }
    }
    Ok(())
}

fn execute_docs(command: DocsCommand, state: &ShellState) -> Result<(), String> {
    match command {
        DocsCommand::Build => {
            let metadata = ResolvedMetadata::resolve(state);
            metadata.validate(
                &ValidationPlan::new("build documentation")
                    .app_local_tools(&[AppToolRequirement::Rust])
                    .workspace(),
            )?;
            let mut client = connect_client_service(state)?;
            content_commands::open_workspace_in_service(&mut client, state)?;
            let event = send_client_action(
                &mut client,
                ClientAction::Workspace(WorkspaceAction::BuildDocumentation),
            )?;
            client_commands::print_service_operation_message(event);
            println!("hint: open the installed documentation with `docs open`");
        }
        DocsCommand::Publish => {
            let metadata = ResolvedMetadata::resolve(state);
            metadata.validate(&ValidationPlan::new("publish documentation").workspace())?;
            let mut client = connect_client_service(state)?;
            content_commands::open_workspace_in_service(&mut client, state)?;
            let event = send_client_action(
                &mut client,
                ClientAction::Workspace(WorkspaceAction::PublishDocumentation),
            )?;
            client_commands::print_service_operation_message(event);
        }
        DocsCommand::Path { topic } => println!(
            "{}",
            documentation::path(state.active_paths()?, topic.as_deref())?.display()
        ),
        DocsCommand::Open { topic } => println!(
            "{}",
            documentation::open(state.active_paths()?, topic.as_deref())?.display()
        ),
    }
    Ok(())
}

fn execute_script_command(command: ScriptCommand, state: &mut ShellState) -> Result<(), String> {
    let ScriptCommand::Run { name, dry_run } = command;
    run_script(&name, dry_run, state)
}

fn execute_provider(command: ProviderCommand, state: &ShellState) -> Result<(), String> {
    match command {
        ProviderCommand::Git { command } => execute_git_provider(command, state),
    }
}

fn execute_git_provider(command: GitProviderCommand, state: &ShellState) -> Result<(), String> {
    match command {
        GitProviderCommand::Status => {
            println!("Git Provider");
            println!();
            match git_provider::linked_path(state.installation())? {
                Some(path) => println!("Linked path: {}", path.display()),
                None => println!("Linked path: none"),
            }
            match git_provider::resolve(state.installation()) {
                Ok(provider) => {
                    println!("Status: ready");
                    println!("Path: {}", provider.path().display());
                    println!("Source: {}", provider.source().label());
                }
                Err(error) => {
                    println!("Status: not configured");
                    println!("Error: {error}");
                    println!();
                    println!("Next");
                    println!("  provider git link /path/to/git");
                }
            }
            Ok(())
        }
        GitProviderCommand::Link { path } => {
            let provider = git_provider::link(state.installation(), &path)?;
            println!("Git Provider");
            println!();
            println!("Status: linked");
            println!("Path: {}", provider.path().display());
            Ok(())
        }
        GitProviderCommand::Clear => {
            let removed = git_provider::clear(state.installation())?;
            println!("Git Provider");
            println!();
            println!(
                "Status: {}",
                if removed {
                    "linked path cleared"
                } else {
                    "no linked path was set"
                }
            );
            println!("Next");
            println!("  provider git status");
            Ok(())
        }
    }
}

fn execute_diagnostics(command: DiagnosticsCommand, state: &ShellState) -> Result<(), String> {
    match command {
        DiagnosticsCommand::Status => {
            println!("Diagnostics");
            println!();
            println!("Status");
            println!(
                "  Local diagnostics: {}",
                diagnostics::local_directory(state.installation()).display()
            );
            match diagnostics::current_run()
                .or_else(|| diagnostics::latest_run(state.installation()))
            {
                Some(run) => {
                    println!("  Latest run: {}", run.run_id());
                    println!("    directory: {}", run.run_dir().display());
                    println!("    metadata: {}", run.metadata_path().display());
                    println!("    log: {}", run.log_path().display());
                }
                None => println!("  Latest run: none"),
            }
            println!(
                "  Automatic capture: {}",
                if diagnostics::auto_capture_enabled() {
                    "active"
                } else {
                    "off for this process"
                }
            );
            match diagnostics::upload_setting() {
                Some(mode) => println!("  Upload request: {mode}"),
                None => println!("  Upload request: off"),
            }
            println!();
            println!("Next");
            println!("  rerun the launch with --send-diagnostics to capture a local run");
            println!("  diagnostics upload --dry-run");
        }
        DiagnosticsCommand::Upload { dry_run } => {
            let report = diagnostics::upload(state.installation(), &UploadOptions { dry_run })?;
            println!("Diagnostics Upload");
            println!();
            println!("Status");
            println!("  Run: {}", report.run().run_id());
            println!("    directory: {}", report.run().run_dir().display());
            println!("    metadata: {}", report.run().metadata_path().display());
            println!("    log: {}", report.run().log_path().display());
            println!("  Transport: {}", report.transport());
            if report.dry_run() {
                println!("  Mode: dry-run");
                println!("  Upload: not sent");
                println!();
                println!("Next");
                println!("  diagnostics upload");
            } else if report.sent() {
                println!("  Upload: sent");
            } else {
                println!("  Upload: not sent");
            }
        }
    }
    Ok(())
}

pub(crate) fn run_script(name: &str, dry_run: bool, state: &mut ShellState) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') || name == "." || name == ".." {
        return Err("script name must be a simple filename stem".to_owned());
    }
    let path = find_script(state, name)?;
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read '{}': {error}", path.display()))?;
    for (index, line) in source.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        println!("{}:{}: {line}", path.display(), index + 1);
        diagnostics::event(format!("script {name}:{}: {line}", index + 1));
        if dry_run {
            continue;
        }
        let args = shlex::split(line)
            .ok_or_else(|| format!("invalid quoting at {}:{}", path.display(), index + 1))?;
        let parsed = ShellCommand::try_parse_from(
            std::iter::once("vapor").chain(args.iter().map(String::as_str)),
        )
        .map_err(|error| error.to_string())?;
        if !script_command_allowed(&parsed) {
            return Err(
                "scripts may not invoke scripts, exit the host shell, perform real publishes, delete Workshop items, send diagnostics, or apply IDE repairs"
                    .to_owned(),
            );
        }
        diagnostics::step(format!("script {name}:{}: {line}", index + 1));
        execute(parsed, state)?;
        diagnostics::event(format!("script {name}:{} complete", index + 1));
    }
    Ok(())
}

fn find_script(state: &ShellState, name: &str) -> Result<PathBuf, String> {
    let filename = format!("{name}.vapor");
    let mut candidates = Vec::new();
    if let Ok(paths) = state.active_paths() {
        candidates.push(
            paths
                .source()
                .root()
                .join("resources/vapor/vapor-scripts")
                .join(&filename),
        );
    }
    candidates.push(
        state
            .installation()
            .root()
            .join("resources/vapor/vapor-scripts")
            .join(filename),
    );

    for path in &candidates {
        if path.is_file() {
            return Ok(path.clone());
        }
    }

    let searched = candidates
        .iter()
        .map(|path| format!("  - {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!("script not found: {name}\nsearched:\n{searched}"))
}

fn script_command_allowed(command: &ShellCommand) -> bool {
    !matches!(
        command,
        ShellCommand::Script { .. }
            | ShellCommand::Exit
            | ShellCommand::Root {
                command: RootCommand::Publish { dry_run: false, .. }
            }
            | ShellCommand::Content {
                command: ContentCommand::Create { dry_run: false, .. },
            }
            | ShellCommand::Content {
                command: ContentCommand::Publish { dry_run: false, .. },
            }
            | ShellCommand::Content {
                command: ContentCommand::Delete { dry_run: false, .. },
            }
            | ShellCommand::Ide {
                command: IdeCommand::Repair { dry_run: false },
            }
            | ShellCommand::Diagnostics {
                command: DiagnosticsCommand::Upload { dry_run: false },
            }
    )
}

pub(crate) fn print_warnings(warnings: Vec<String>) {
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
}

#[cfg(test)]
mod client_launch_tests {
    use super::client_commands::require_workshop_id;
    use super::model::client_launch_arguments;
    use super::{ClientCommand, ClientLaunchMode, RootCommand, ShellCommand};
    use clap::Parser;
    use std::ffi::OsString;

    #[test]
    fn workshop_subscription_requires_a_decimal_published_file_id() {
        assert!(require_workshop_id("3762162548").is_ok());
        assert!(require_workshop_id("").is_err());
        assert!(require_workshop_id("0").is_err());
        assert!(require_workshop_id("author/content").is_err());
    }

    #[test]
    fn client_launch_defaults_to_the_studio_entrypoint_mode() {
        let parsed = ShellCommand::try_parse_from(["vapor", "client", "launch"]).unwrap();
        assert!(matches!(
            parsed,
            ShellCommand::Client {
                command: ClientCommand::Launch {
                    mode: ClientLaunchMode::Studio,
                    ..
                }
            }
        ));
    }

    #[test]
    fn clean_is_a_first_class_workspace_service_operation() {
        let parsed = ShellCommand::try_parse_from(["vapor", "clean"]).unwrap();
        assert!(matches!(parsed, ShellCommand::Clean));
    }

    #[test]
    fn composition_lifecycle_commands_are_first_class_cli_operations() {
        for (operation, expected) in [
            ("install", "install"),
            ("verify", "verify"),
            ("repair", "repair"),
            ("update", "update"),
            ("launch", "launch"),
        ] {
            let parsed = ShellCommand::try_parse_from([
                "vapor",
                "composition",
                operation,
                "demo-composition",
            ])
            .unwrap();
            let ShellCommand::Composition { command } = parsed else {
                panic!("composition operation should parse as a composition command");
            };
            assert_eq!(command.operation_name(), expected);
            assert_eq!(command.composition_id(), Some("demo-composition"));
        }
    }

    #[test]
    fn composition_list_needs_no_identifier() {
        let parsed = ShellCommand::try_parse_from(["vapor", "composition", "list"]).unwrap();
        let ShellCommand::Composition { command } = parsed else {
            panic!("composition list should parse as a composition command");
        };
        assert_eq!(command.operation_name(), "list");
        assert_eq!(command.composition_id(), None);
    }

    #[test]
    fn client_launch_accepts_each_supported_entrypoint_mode() {
        for (argument, expected) in [
            ("launcher", ClientLaunchMode::Launcher),
            ("studio", ClientLaunchMode::Studio),
            ("play", ClientLaunchMode::Launcher),
            ("shell", ClientLaunchMode::Shell),
            ("installer", ClientLaunchMode::Installer),
        ] {
            let parsed =
                ShellCommand::try_parse_from(["vapor", "client", "launch", argument]).unwrap();
            assert!(matches!(
                parsed,
                ShellCommand::Client {
                    command: ClientCommand::Launch { mode, .. }
                } if mode == expected
            ));
        }
    }

    #[test]
    fn client_launcher_mode_uses_the_entrypoints_player_launch_route() {
        assert_eq!(
            client_launch_arguments(ClientLaunchMode::Launcher, Vec::new()),
            [OsString::from("play")]
        );
    }

    #[test]
    fn client_launch_forwards_mode_and_gui_arguments_to_entrypoint() {
        let parsed = ShellCommand::try_parse_from([
            "vapor",
            "client",
            "launch",
            "studio",
            "--",
            "--workspace",
            "/tmp/loo-cast",
        ])
        .unwrap();
        let ShellCommand::Client {
            command: ClientCommand::Launch { mode, arguments },
        } = parsed
        else {
            panic!("client launch should parse as an entrypoint launch");
        };

        assert_eq!(
            client_launch_arguments(mode, arguments),
            [
                OsString::from("studio"),
                OsString::from("--workspace"),
                OsString::from("/tmp/loo-cast"),
            ]
        );
    }

    #[test]
    fn root_build_release_selects_the_optimized_profile_without_deploying() {
        let parsed =
            ShellCommand::try_parse_from(["vapor", "root", "build", "--host-only", "--release"])
                .unwrap();

        assert!(matches!(
            parsed,
            ShellCommand::Root {
                command: RootCommand::Build {
                    release: true,
                    host_only: true,
                    ..
                }
            }
        ));
    }
}
