//! Shell adapter for Vapor Core's installation-owned Git provider.

use crate::discovery::InstallationPaths;
pub(crate) use vapor_core::git_provider::GitProvider;
use std::path::Path;

pub(crate) fn resolve(installation: &InstallationPaths) -> Result<GitProvider, String> {
    vapor_core::git_provider::resolve(
        &installation.user_data_root(),
        &installation.state_dir(),
    )
}

pub(crate) fn link(installation: &InstallationPaths, path: &Path) -> Result<GitProvider, String> {
    vapor_core::git_provider::link(
        &installation.user_data_root(),
        &installation.state_dir(),
        path,
    )
}

pub(crate) fn clear(installation: &InstallationPaths) -> Result<bool, String> {
    vapor_core::git_provider::clear(&installation.state_dir())
}

pub(crate) fn linked_path(installation: &InstallationPaths) -> Result<Option<std::path::PathBuf>, String> {
    vapor_core::git_provider::linked_path(&installation.state_dir())
}
