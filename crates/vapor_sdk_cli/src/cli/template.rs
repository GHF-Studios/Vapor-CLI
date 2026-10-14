//! Clap commands for SDK template workflows.

use clap::Subcommand;
use vapor_sdk_core as core;

#[derive(Subcommand)]
pub(super) enum TemplateCommand {
    List,
    Info,
}

impl TemplateCommand {
    pub(super) fn into_core(self) -> core::TemplateCommand {
        match self {
            Self::List => core::TemplateCommand::List,
            Self::Info => core::TemplateCommand::Info,
        }
    }
}
