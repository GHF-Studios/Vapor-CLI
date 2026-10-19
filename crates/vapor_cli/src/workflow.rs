//! Cargo workflows routed through the Steam-installed Vapor setup.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};
use vapor_client_provider::discovery::EnvironmentPaths;
pub(crate) use vapor_client_provider::host_runtime_target;

/// Dynamic Cargo workspace selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSelection {
    /// Every Cargo workspace discovered for this source root.
    All,
    /// Cargo workspaces that declare installed application binaries.
    Installable,
    /// One named Cargo workspace discovered from the active source root.
    One(String),
}

impl std::str::FromStr for ProjectSelection {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        if value == "all" {
            return Ok(Self::All);
        }
        if value == "installable" {
            return Ok(Self::Installable);
        }
        if value.is_empty()
            || value.chars().any(|character| {
                !character.is_ascii_lowercase() && !character.is_ascii_digit() && character != '-'
            })
        {
            return Err(
                "project must be `all` or a lowercase kebab-case Cargo workspace name".to_owned(),
            );
        }
        Ok(Self::One(value.to_owned()))
    }
}

/// Supported Cargo operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoWorkflow {
    /// Apply Rustfmt.
    Fmt,
    /// Type-check all workspace targets.
    Check,
    /// Test all workspace targets.
    Test,
    /// Build every workspace package.
    Build,
    /// Build every workspace package with Cargo's optimized release profile.
    BuildRelease,
    /// Check formatting, compile, test, lint, and build documentation.
    Validate,
}

impl CargoWorkflow {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Fmt => "fmt",
            Self::Check => "check",
            Self::Test => "test",
            Self::Build => "build",
            Self::BuildRelease => "build --release",
            Self::Validate => "validate",
        }
    }
}

pub(crate) fn managed_path(paths: &EnvironmentPaths) -> Result<OsString, String> {
    let installation = paths.installation();
    let root = installation.root();
    let mut entries = Vec::<PathBuf>::new();
    // Cargo is an app-local rustup proxy; put the matching app-local compiler
    // first as well, or it can pick up a host rustc from the inherited PATH.
    entries.push(
        crate::app_local_tools::inspect(installation)
            .rust()
            .path()
            .to_path_buf(),
    );
    entries.push(installation.binaries().to_path_buf());
    entries.push(root.join("bin"));
    if let Some(existing) = env::var_os("PATH") {
        entries
            .extend(env::split_paths(&existing).filter(|entry| !is_raw_app_tool_path(root, entry)));
    }
    env::join_paths(entries).map_err(|error| format!("failed to construct Vapor PATH: {error}"))
}

fn is_raw_app_tool_path(root: &Path, entry: &Path) -> bool {
    let raw_exact = [
        root.join("cargo-home/bin"),
        root.join("rustup/bin"),
        root.join("tools/steamcmd"),
        root.join("tools/zig"),
        root.join("tools/cross/bin"),
        root.join("tools/llvm-mingw/bin"),
    ];
    if raw_exact.iter().any(|raw| entry == raw) {
        return true;
    }

    let toolchains = root.join("rustup-home/toolchains");
    entry.starts_with(&toolchains) && entry.file_name().is_some_and(|name| name == "bin")
}
