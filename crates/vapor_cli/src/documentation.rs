//! Build, locate, and open installed Vapor documentation.

use crate::{discovery::EnvironmentPaths, workspace::WorkspaceManifest};
use std::{path::PathBuf, process::Command};

/// Build Rustdoc for every workspace declared by the distribution manifest.
pub fn build(paths: &EnvironmentPaths, manifest: &WorkspaceManifest) -> Result<PathBuf, String> {
    vapor_client_provider::documentation::build(paths, manifest)
}

/// Resolve an installed documentation section or the aggregate index.
pub fn path(paths: &EnvironmentPaths, topic: Option<&str>) -> Result<PathBuf, String> {
    let root = paths.installation().root().join("docs");
    let candidate = topic.map_or_else(
        || root.join("index.html"),
        |name| root.join(name).join("index.html"),
    );
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(format!(
            "documentation is not built: {}",
            candidate.display()
        ))
    }
}

/// Open documentation without blocking the Vapor command loop.
pub fn open(paths: &EnvironmentPaths, topic: Option<&str>) -> Result<PathBuf, String> {
    let document = path(paths, topic)?;
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    } else if cfg!(target_os = "macos") {
        Command::new("open")
    } else {
        Command::new("xdg-open")
    };
    command
        .arg(&document)
        .spawn()
        .map_err(|error| format!("failed to open docs: {error}"))?;
    Ok(document)
}
