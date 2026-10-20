//! Public command vocabulary accepted by Vapor CLI and its shell.

use crate::{metadata::MetadataFormat, workflow::ProjectSelection};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "vapor",
    bin_name = "vapor",
    after_help = "Run `help COMMAND` for details on one command group.\nNormal tester launches start with `launch default`; developer work starts with script-prepared tooling, then `source open SOURCE`."
)]
/// Commands accepted by the Vapor shell and its narrow host facades.
pub enum ShellCommand {
    /// Launch a playable Vapor composition.
    Launch {
        /// Run the nearest workspace's default source configuration from the
        /// current directory.
        #[arg(long)]
        current_workspace: bool,
        /// Mark this local development run as a profiling session.
        /// This starts telemetry session capture; use the Devtools/Tracy UI for analysis.
        #[arg(long)]
        profiling: bool,
        /// Installed composition to launch. Omit to launch the app's default composition.
        #[command(subcommand)]
        command: Option<LaunchCommand>,
    },

    /// Build and locally deploy the Vapor Client workspace from source.
    Client {
        /// Client operation.
        #[command(subcommand)]
        command: ClientCommand,
    },

    /// Open, index, inspect, sync, or repair authored source roots.
    Source {
        /// Source operation.
        #[command(subcommand)]
        command: SourceCommand,
    },

    /// Print the Steam installation/app root.
    Installation,

    /// Print the directory containing the running Vapor shell binary.
    Binaries,

    /// Print the installation's conventional library directory, when present.
    Libraries,

    /// Show resolved source, installation, setup, and Cargo metadata.
    Metadata {
        /// Output representation for people or automation.
        #[arg(long, value_enum, default_value_t)]
        format: MetadataFormat,
    },

    /// Format Cargo workspaces through app-local Rust/Cargo.
    Fmt {
        /// Cargo workspace name to format, or `all`.
        #[arg(long, value_name = "PROJECT", default_value = "all")]
        project: ProjectSelection,
    },

    /// Check Cargo workspaces through app-local Rust/Cargo.
    Check {
        /// Cargo workspace name to check, or `all`.
        #[arg(long, value_name = "PROJECT", default_value = "all")]
        project: ProjectSelection,
    },

    /// Test Cargo workspaces through app-local Rust/Cargo.
    Test {
        /// Cargo workspace name to test, or `all`.
        #[arg(long, value_name = "PROJECT", default_value = "all")]
        project: ProjectSelection,
    },

    /// Build Cargo workspaces through app-local Rust/Cargo.
    Build {
        /// Cargo workspace name to build, or `all`.
        #[arg(long, value_name = "PROJECT", default_value = "all")]
        project: ProjectSelection,
    },

    /// Run formatting, checking, tests, Clippy, and Rustdoc.
    Validate {
        /// Cargo workspace name to validate, or `all`.
        #[arg(long, value_name = "PROJECT", default_value = "all")]
        project: ProjectSelection,
    },

    /// Build, locate, or open installed documentation.
    Docs {
        /// Documentation operation.
        #[command(subcommand)]
        command: DocsCommand,
    },

    /// Inspect or repair project-local IDE settings.
    Ide {
        /// IDE setup operation.
        #[command(subcommand)]
        command: IdeCommand,
    },

    /// Build, package, or publish the application/depot root.
    Root {
        /// Root application operation.
        #[command(subcommand)]
        command: RootCommand,
    },

    /// Inspect or operate on typed Workshop/content nodes.
    Content {
        /// Content operation.
        #[command(subcommand)]
        command: ContentCommand,
    },

    /// Run a source-controlled sequence of Vapor commands.
    Script {
        /// Script operation.
        #[command(subcommand)]
        command: ScriptCommand,
    },

    /// Inspect or link external developer providers.
    Provider {
        /// Provider operation.
        #[command(subcommand)]
        command: ProviderCommand,
    },

    /// Open or inspect tools managed by the pinned Vapor toolchain.
    Toolchain {
        /// Toolchain operation.
        #[command(subcommand)]
        command: ToolchainCommand,
    },

    /// Author workspaces and content, and manage SDK release operations.
    Sdk {
        /// Arguments accepted by the SDK command surface.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },

    /// Inspect or ship private-test launch diagnostics.
    Diagnostics {
        /// Diagnostics operation.
        #[command(subcommand)]
        command: DiagnosticsCommand,
    },

    /// Exit the Vapor sub-shell.
    #[command(alias = "quit")]
    Exit,
}

/// Playable launch targets.
#[derive(Debug, Subcommand)]
pub enum LaunchCommand {
    /// Launch the app manifest's default playable composition through Client.
    Default,
    /// Launch the composition selected in Studio through Client.
    Selected,
}

/// Utilities installed by the version-pinned Vapor toolchain.
#[derive(Debug, Subcommand)]
pub enum ToolchainCommand {
    /// Open Tracy for live capture and trace inspection.
    Tracy,
}

#[derive(Debug, Subcommand)]
/// Vapor Client deployment operations.
pub enum ClientCommand {
    /// Launch the locally deployed Steam entrypoint without Steam.
    Launch {
        /// Entrypoint mode; defaults to the Studio home screen.
        #[arg(value_enum, default_value = "studio")]
        mode: ClientLaunchMode,
        /// Arguments forwarded to the selected entrypoint mode after `--`.
        #[arg(last = true, allow_hyphen_values = true)]
        arguments: Vec<std::ffi::OsString>,
    },

    /// Deploy the active Vapor Client workspace.
    Deploy {
        /// Deployment target.
        #[command(subcommand)]
        command: ClientDeployCommand,
    },
}

/// Modes accepted by the deployed Vapor Steam entrypoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ClientLaunchMode {
    /// Open the Vapor Studio home screen.
    Studio,
    /// Play the active profile composition.
    Play,
    /// Open the Vapor command shell.
    Shell,
    /// Open the installation/setup flow.
    Installer,
}

pub(super) fn client_launch_arguments(
    mode: ClientLaunchMode,
    arguments: Vec<std::ffi::OsString>,
) -> Vec<std::ffi::OsString> {
    let mode = match mode {
        ClientLaunchMode::Studio => "studio",
        ClientLaunchMode::Play => "play",
        ClientLaunchMode::Shell => "shell",
        ClientLaunchMode::Installer => "installer",
    };
    std::iter::once(std::ffi::OsString::from(mode))
        .chain(arguments)
        .collect()
}

#[derive(Debug, Subcommand)]
/// Local Vapor Client deployment targets.
pub enum ClientDeployCommand {
    /// Build and promote the source-authoritative local Vapor App Instance.
    Local,
}

/// Private-test diagnostics operations.
#[derive(Debug, Subcommand)]
pub enum DiagnosticsCommand {
    /// Report local diagnostics capture and upload state.
    Status,
    /// Upload the current/latest local diagnostics run through the configured transport.
    #[command(alias = "submit", alias = "send")]
    Upload {
        /// Preview the local run and transport without sending it.
        #[arg(long)]
        dry_run: bool,
    },
}

/// External developer provider operations.
#[derive(Debug, Subcommand)]
pub enum ProviderCommand {
    /// Inspect or link the developer Git provider.
    Git {
        /// Git provider operation.
        #[command(subcommand)]
        command: GitProviderCommand,
    },
}

/// Developer Git provider operations.
#[derive(Debug, Subcommand)]
pub enum GitProviderCommand {
    /// Report how Git is currently resolved.
    Status,
    /// Persist an explicit Git executable path.
    Link {
        /// Git executable path.
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
    /// Remove the persisted Git executable path.
    Clear,
}

/// Documentation operations.
#[derive(Debug, Subcommand)]
pub enum DocsCommand {
    /// Build all declared Rustdoc into the installation.
    Build,
    /// Publish the curated public documentation site to Vapor Docs.
    Publish,
    /// Print an installed documentation path.
    Path {
        /// Optional documentation section.
        topic: Option<String>,
    },
    /// Open installed documentation without blocking the shell.
    Open {
        /// Optional documentation section.
        topic: Option<String>,
    },
}

/// Authored source-root operations.
#[derive(Debug, Subcommand)]
pub enum SourceCommand {
    /// Create a new authored source workspace from a Vapor template.
    Init {
        /// Template to create.
        #[arg(value_enum)]
        template: SourceTemplate,
        /// Target workspace path. It must be empty or absent.
        #[arg(value_name = "PATH")]
        path: PathBuf,
        /// Owning organization namespace, such as a Steam publisher or studio slug.
        #[arg(long)]
        organization: String,
        /// Workspace slug.
        #[arg(long)]
        name: String,
        /// Steam AppID used in generated Workshop metadata. Defaults to the installed app.
        #[arg(long)]
        app_id: Option<u32>,
    },
    /// Report active source and indexed source state.
    Status,
    /// Open an indexed source name or external source path.
    Open {
        /// Indexed source name, full source ID, or filesystem path.
        #[arg(value_name = "SOURCE")]
        source: String,
    },
    /// Close the active source while keeping the app shell alive.
    Close,
    /// List indexed source roots.
    List,
    /// Index a source root without opening it.
    Add {
        /// Source path; defaults to the process working directory.
        #[arg(value_name = "SOURCE_PATH")]
        path: Option<PathBuf>,
    },
    /// Remove an indexed source by local name or full ID.
    Remove {
        /// Indexed source name or full source ID.
        #[arg(value_name = "SOURCE")]
        source: String,
    },
    /// Synchronize authored source through controlled source providers.
    Sync,
    /// Inspect or repair source registration and authored metadata.
    Repair {
        /// Apply safe source metadata repairs instead of only reporting them.
        #[arg(long)]
        write: bool,
    },
}

/// Built-in source workspace templates.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SourceTemplate {
    /// Basic engine, game, and packagepack content workspace.
    BasicContent,
}

/// Project-local IDE setup operations.
#[derive(Debug, Subcommand)]
pub enum IdeCommand {
    /// Report RustRover/JetBrains project-local setup state.
    Status,
    /// Write project-local settings for app-local Rust/Cargo.
    Repair {
        /// Preview IDE files without writing them.
        #[arg(long)]
        dry_run: bool,
    },
}

/// Complete application/depot root operations.
#[derive(Debug, Subcommand)]
pub enum RootCommand {
    /// Build every root application project into app-root build outputs.
    Build {
        /// Build optimized release artifacts without promoting them.
        #[arg(long)]
        release: bool,
        /// Rust target triple for root application binaries. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Build the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Build only Cargo's host target for a local smoke pass.
        #[arg(long)]
        host_only: bool,
    },
    /// Promote already-built root application outputs into the local app root.
    Deploy {
        /// Skip rebuilding installed documentation.
        #[arg(long)]
        skip_docs: bool,
        /// Rust target triple for root application binaries. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Deploy the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Deploy only Cargo's host target for a local smoke pass.
        #[arg(long)]
        host_only: bool,
    },
    /// Assemble and smoke-check the local application/depot package.
    Package {
        /// Runtime target triple to stage launch scripts for. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Stage the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Stage only the host target for a local smoke pass.
        #[arg(long)]
        host_only: bool,
    },
    /// Validate, build, stage, preview, or upload the complete Steam app/depot.
    Publish {
        /// Dedicated Steam build account. Required for real uploads.
        #[arg(long)]
        account: Option<String>,
        /// Existing non-default beta branch; defaults to the distribution manifest.
        #[arg(long)]
        branch: Option<String>,
        /// Rust target triple for dry-run/custom root application binaries. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Build and stage the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Build and stage only the host target for a dry-run/local smoke pass.
        #[arg(long)]
        host_only: bool,
        /// Use already-promoted app binaries for dry-run previews only.
        #[arg(long)]
        skip_build: bool,
        /// Internal Steam build description.
        #[arg(long, default_value = "Vapor development build")]
        description: String,
        /// Generate staging and a preview VDF without uploading.
        #[arg(long)]
        dry_run: bool,
        /// Confirm the real network upload.
        #[arg(long)]
        yes: bool,
    },
}

/// Typed content-node operations.
#[derive(Debug, Subcommand)]
pub enum ContentCommand {
    /// Report the active Workshop/content node.
    Status,
    /// List source artifacts and installed content state.
    List,
    /// Validate source content metadata, dependencies, conflicts, and publication intent.
    Validate {
        /// Artifact ID, local name, or PublishedFileId. Omit to validate all.
        #[arg(value_name = "ARTIFACT")]
        artifact: Option<String>,
    },
    /// Build the active content workspace through app-local Cargo.
    Build {
        /// Rust target triple for content runtime outputs. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Build the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Build only Cargo's host target for a local smoke pass.
        #[arg(long)]
        host_only: bool,
    },
    /// Build and locally install source content without Workshop publication.
    Deploy {
        /// Artifact ID, local name, or PublishedFileId.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        artifact: String,
        /// Select the deployed artifact as the active packagepack.
        #[arg(long)]
        select: bool,
        /// Rust target triple for content runtime outputs. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Deploy the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Deploy only Cargo's host target for a local smoke pass.
        #[arg(long)]
        host_only: bool,
    },
    /// Stage a content package under the app root.
    Package {
        /// Artifact ID, local name, or PublishedFileId.
        #[arg(value_name = "ARTIFACT")]
        artifact: String,
        /// Rust target triple for dry-run/custom content runtime outputs. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Package the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Package only the host target for a dry-run/local smoke pass.
        #[arg(long)]
        host_only: bool,
        /// Preview package output without writing it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Acquire content into the app-owned cache.
    Acquire {
        /// Authored artifact selector or Steam Workshop PublishedFileId.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: String,
    },
    /// Subscribe to a Steam Workshop item and acquire it through the shared Client service.
    Subscribe {
        /// Steam Workshop PublishedFileId.
        #[arg(value_name = "PUBLISHED_FILE_ID")]
        target: String,
    },
    /// Download content into the app-owned cache.
    Download {
        /// Authored artifact selectors or Steam Workshop PublishedFileIds.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID", required = true, num_args = 1..)]
        targets: Vec<String>,
    },
    /// Install source or cached content into the app root.
    Install {
        /// Artifact ID, local name, PublishedFileId, or cached Workshop ID.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: String,
        /// Steam account used when Workshop content must be downloaded.
        #[arg(long)]
        account: Option<String>,
    },
    /// Update installed content from source or cache.
    Update {
        /// Artifact ID, local name, PublishedFileId, or cached Workshop ID. Omit to update all.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: Option<String>,
    },
    /// Verify installed content fingerprints and receipts.
    Verify {
        /// Artifact ID, local name, PublishedFileId, or cached Workshop ID. Omit to verify all.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: Option<String>,
    },
    /// Report the selected packagepack.
    Selected,
    /// Select an installed packagepack for play.
    Select {
        /// Installed packagepack artifact ID or PublishedFileId.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: String,
    },
    /// Clear the selected packagepack.
    Deselect,
    /// Repair corrupted installed content from source or cache.
    Repair {
        /// Artifact ID, local name, PublishedFileId, or cached Workshop ID. Omit to repair all.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: Option<String>,
    },
    /// Disable installed content without deleting it.
    Disable {
        /// Artifact ID, local name, PublishedFileId, or cached Workshop ID.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: String,
    },
    /// Enable disabled content.
    Enable {
        /// Artifact ID, local name, PublishedFileId, or cached Workshop ID.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: String,
    },
    /// Uninstall content and remove installed-state records.
    Uninstall {
        /// Artifact ID, local name, PublishedFileId, or cached Workshop ID.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: String,
    },
    /// Preview creation of a new Workshop item.
    Create {
        /// Artifact ID or local name.
        #[arg(value_name = "ARTIFACT")]
        artifact: String,
        /// Dedicated Steam publishing account. Required for real creation.
        #[arg(long)]
        account: Option<String>,
        /// Rust target triple for dry-run/custom content runtime outputs. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Package the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Package only the host target for a dry-run/local smoke pass.
        #[arg(long)]
        host_only: bool,
        /// Preview the SteamUGC create request without changing authority.
        #[arg(long)]
        dry_run: bool,
        /// Confirm the real Workshop create/upload.
        #[arg(long)]
        yes: bool,
    },
    /// Publish or preview a Workshop item update.
    Publish {
        /// Artifact IDs or local names.
        #[arg(value_name = "ARTIFACT", required = true, num_args = 1..)]
        artifacts: Vec<String>,
        /// Dedicated Steam publishing account. Required for real uploads.
        #[arg(long)]
        account: Option<String>,
        /// Rust target triple for dry-run/custom content runtime outputs. May be repeated.
        #[arg(long, value_name = "TARGET")]
        target: Vec<String>,
        /// Package the manifest runtime target matrix. This is the default when declared.
        #[arg(long)]
        release_targets: bool,
        /// Package only the host target for a dry-run/local smoke pass.
        #[arg(long)]
        host_only: bool,
        /// Workshop update note.
        #[arg(long)]
        change_note: Option<String>,
        /// Generate package and provider preview without uploading.
        #[arg(long)]
        dry_run: bool,
        /// Confirm the real network upload.
        #[arg(long)]
        yes: bool,
    },
    /// Preview deletion or retirement of a Workshop item.
    Delete {
        /// Artifact ID, local name, or PublishedFileId.
        #[arg(value_name = "ARTIFACT_OR_WORKSHOP_ID")]
        target: String,
        /// Preview the SteamUGC delete request without changing authority.
        #[arg(long)]
        dry_run: bool,
        /// Confirm a real authority-changing delete request.
        #[arg(long)]
        yes: bool,
    },
}

/// Vapor command-script operations.
#[derive(Debug, Subcommand)]
pub enum ScriptCommand {
    /// Run `resources/vapor/vapor-scripts/<NAME>.vapor`.
    Run {
        /// Script filename stem under `resources/vapor/vapor-scripts`.
        name: String,
        /// Print commands without executing them.
        #[arg(long)]
        dry_run: bool,
    },
}
