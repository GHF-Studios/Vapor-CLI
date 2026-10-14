//! Clap commands for authored non-root packs.

use super::args::{ContentSource, ContentType, child};
use clap::Subcommand;
use vapor_sdk_core as core;

#[derive(Subcommand)]
pub(super) enum PackCommand {
    /// List packs from one content source.
    List { source: ContentSource },
    /// Show local or discovered status for one pack.
    Status { pack_id: String },
    /// Compute or display the deterministic pack fingerprint.
    Fingerprint { pack_id: String },
    /// Inspect pack metadata, graph hints, and authoring state.
    Inspect { pack_id: String },
    /// Validate pack metadata and composition invariants.
    Validate { pack_id: String },
    /// Create a new pack source project.
    New { pack_id: String },
    /// Initialize the current empty directory as a pack source project.
    Init { pack_id: String },
    /// Add child content to an authored pack.
    Add {
        pack_id: String,
        child_content_type: ContentType,
        child_content_id: String,
    },
    /// Remove child content from an authored pack.
    Remove {
        pack_id: String,
        child_content_type: ContentType,
        child_content_id: String,
    },
    /// Select active child content inside an authored pack.
    Select {
        pack_id: String,
        child_content_type: ContentType,
        child_content_id: String,
    },
    /// Keep child content present but inactive inside an authored pack.
    Unselect {
        pack_id: String,
        child_content_type: ContentType,
        child_content_id: String,
    },
    /// Build pack source artifacts through Vapor.
    Build { pack_id: String },
    /// Package a pack for distribution.
    Package { pack_id: String },
    /// Publish a pack through configured content release channels.
    Publish { pack_id: String },
}

impl PackCommand {
    pub(super) fn into_core(self) -> core::PackCommand {
        match self {
            Self::List { source } => read(core::ContentReadCommand::List { source }),
            Self::Status { pack_id } => read(core::ContentReadCommand::Status {
                content_id: pack_id,
            }),
            Self::Fingerprint { pack_id } => read(core::ContentReadCommand::Fingerprint {
                content_id: pack_id,
            }),
            Self::Inspect { pack_id } => read(core::ContentReadCommand::Inspect {
                content_id: pack_id,
            }),
            Self::Validate { pack_id } => read(core::ContentReadCommand::Validate {
                content_id: pack_id,
            }),
            Self::New { pack_id } => author(core::SourceAuthoringCommand::New {
                content_id: pack_id,
            }),
            Self::Init { pack_id } => author(core::SourceAuthoringCommand::Init {
                content_id: pack_id,
            }),
            Self::Build { pack_id } => author(core::SourceAuthoringCommand::Build {
                content_id: pack_id,
            }),
            Self::Package { pack_id } => author(core::SourceAuthoringCommand::Package {
                content_id: pack_id,
            }),
            Self::Publish { pack_id } => author(core::SourceAuthoringCommand::Publish {
                content_id: pack_id,
            }),
            Self::Add {
                pack_id,
                child_content_type,
                child_content_id,
            } => compose(core::PackCompositionCommand::Add {
                pack_id,
                child: child(child_content_type, child_content_id),
            }),
            Self::Remove {
                pack_id,
                child_content_type,
                child_content_id,
            } => compose(core::PackCompositionCommand::Remove {
                pack_id,
                child: child(child_content_type, child_content_id),
            }),
            Self::Select {
                pack_id,
                child_content_type,
                child_content_id,
            } => compose(core::PackCompositionCommand::Select {
                pack_id,
                child: child(child_content_type, child_content_id),
            }),
            Self::Unselect {
                pack_id,
                child_content_type,
                child_content_id,
            } => compose(core::PackCompositionCommand::Unselect {
                pack_id,
                child: child(child_content_type, child_content_id),
            }),
        }
    }
}

fn read(command: core::ContentReadCommand) -> core::PackCommand {
    core::PackCommand::Read(command)
}

fn author(command: core::SourceAuthoringCommand) -> core::PackCommand {
    core::PackCommand::Author(command)
}

fn compose(command: core::PackCompositionCommand) -> core::PackCommand {
    core::PackCommand::Compose(command)
}
