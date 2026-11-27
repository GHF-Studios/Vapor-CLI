use clap::Subcommand;
use vapor_sdk_core as core;

#[derive(Debug, Subcommand)]
pub(crate) enum GitCommand {
    /// Build pinned upstream Git in the disposable, host-specific cache.
    Build,
}

impl GitCommand {
    pub(crate) fn into_core(self) -> core::GitCommand {
        match self {
            Self::Build => core::GitCommand::Build,
        }
    }
}
