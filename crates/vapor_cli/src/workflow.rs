//! Cargo workflows routed through the Steam-installed Vapor setup.

use crate::{
    cross_toolchain,
    discovery::{EnvironmentPaths, ensure_contained},
    workspace::WorkspaceManifest,
};
use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};
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

/// Promote configured build artifacts into the installation's target-specific
/// `bin/<target>` directory for the host target.
///
/// # Errors
///
/// Fails when a configured artifact is missing, escapes installation output,
/// or cannot replace its installed destination.
pub fn promote(paths: &EnvironmentPaths, manifest: &WorkspaceManifest) -> Result<usize, String> {
    promote_for_targets(paths, manifest, &[])
}

/// Promote configured build artifacts for one or more target triples.
///
/// An empty target list uses Cargo's host-default output directory and promotes
/// into `bin/<host-target>`. Explicit targets read Cargo's
/// `output/dev/<workspace>/<target>/debug` output and promote into
/// `bin/<target>`.
///
/// # Errors
///
/// Fails when a configured artifact is missing, escapes installation output,
/// or cannot replace its installed destination.
pub fn promote_for_targets(
    paths: &EnvironmentPaths,
    manifest: &WorkspaceManifest,
    target_triples: &[String],
) -> Result<usize, String> {
    promote_for_targets_in_profile(paths, manifest, target_triples, "dev")
}

/// Promote configured release-profile artifacts into the target-specific app bin.
pub fn promote_release_for_targets(
    paths: &EnvironmentPaths,
    manifest: &WorkspaceManifest,
    target_triples: &[String],
) -> Result<usize, String> {
    promote_for_targets_in_profile(paths, manifest, target_triples, "release")
}

fn promote_for_targets_in_profile(
    paths: &EnvironmentPaths,
    manifest: &WorkspaceManifest,
    target_triples: &[String],
    profile: &str,
) -> Result<usize, String> {
    let targets = RuntimeTarget::many(target_triples)?;
    let installation = paths.installation().root();
    let mut promoted = 0;
    for project in manifest.cargo_projects() {
        for binary in project.binaries() {
            for target in &targets {
                let filename = target.binary_file_name(binary);
                let source = target
                    .build_output_root(installation, project.name(), profile)
                    .join(&filename);
                ensure_contained(installation, &source)?;
                if !source.is_file() {
                    return Err(format!(
                        "built binary is missing for project '{}' target '{}': {}",
                        project.name(),
                        target.triple(),
                        source.display()
                    ));
                }
                let destination = installation
                    .join("bin")
                    .join(target.triple())
                    .join(&filename);
                ensure_contained(installation, &destination)?;
                promote_file(&source, &destination)?;
                let destination_root = destination.parent().ok_or_else(|| {
                    format!(
                        "promoted binary destination has no parent: {}",
                        destination.display()
                    )
                })?;
                let support_dlls = cross_toolchain::copy_windows_runtime_dlls(
                    installation,
                    target.triple(),
                    destination_root,
                )?;
                println!("promoted {} -> {}", source.display(), destination.display());
                if support_dlls > 0 {
                    println!(
                        "promoted {support_dlls} Windows runtime DLL(s) -> {}",
                        destination_root.display()
                    );
                }
                promoted += 1;
            }
        }
        for target in &targets {
            for library in project.libraries(target.triple()) {
                let source = target
                    .build_output_root(installation, project.name(), profile)
                    .join(library);
                ensure_contained(installation, &source)?;
                if !source.is_file() {
                    return Err(format!(
                        "built runtime library is missing for project '{}' target '{}': {}",
                        project.name(),
                        target.triple(),
                        source.display()
                    ));
                }
                let destination = installation.join("bin").join(target.triple()).join(library);
                ensure_contained(installation, &destination)?;
                promote_file(&source, &destination)?;
                println!(
                    "promoted runtime library {} -> {}",
                    source.display(),
                    destination.display()
                );
            }
        }
    }
    for target in &targets {
        let bin_root = installation.join("bin").join(target.triple());
        for legacy in [
            "vapor-devtools",
            "vapor-studio-service",
            "vapor-monitor",
            "vapor-profile",
        ] {
            let path = bin_root.join(target.binary_file_name(legacy));
            if path.is_file() {
                fs::remove_file(&path).map_err(|error| {
                    format!(
                        "failed to remove retired binary '{}': {error}",
                        path.display()
                    )
                })?;
            }
        }
    }
    Ok(promoted)
}

#[derive(Debug, Clone)]
struct RuntimeTarget {
    triple: String,
    explicit: bool,
}

impl RuntimeTarget {
    fn many(target_triples: &[String]) -> Result<Vec<Self>, String> {
        if target_triples.is_empty() {
            return Ok(vec![Self {
                triple: host_runtime_target(),
                explicit: false,
            }]);
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut targets = Vec::new();
        for target in target_triples {
            vapor_client_provider::validate_runtime_target(target)?;
            if !seen.insert(target.clone()) {
                return Err(format!("duplicate runtime target: {target}"));
            }
            targets.push(Self {
                triple: target.clone(),
                explicit: true,
            });
        }
        Ok(targets)
    }

    fn triple(&self) -> &str {
        &self.triple
    }

    fn build_output_root(&self, installation: &Path, project_name: &str, profile: &str) -> PathBuf {
        let root = installation.join("output").join(profile).join(project_name);
        if self.explicit {
            root.join(self.triple()).join(profile)
        } else {
            root.join(profile)
        }
    }

    fn binary_file_name(&self, binary: &str) -> String {
        if self.triple.contains("windows") {
            if binary.ends_with(".exe") {
                binary.to_owned()
            } else {
                format!("{binary}.exe")
            }
        } else {
            binary.to_owned()
        }
    }
}

fn promote_file(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination.parent().ok_or_else(|| {
        format!(
            "binary destination has no parent: {}",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    let temporary = destination.with_extension(format!("tmp-{}", std::process::id()));
    if temporary.exists() {
        fs::remove_file(&temporary)
            .map_err(|error| format!("failed to clear '{}': {error}", temporary.display()))?;
    }
    fs::copy(source, &temporary).map_err(|error| {
        format!(
            "failed to copy '{}' to '{}': {error}",
            source.display(),
            temporary.display()
        )
    })?;
    if destination.exists() {
        fs::remove_file(destination).map_err(|error| {
            format!(
                "failed to replace installed binary '{}': {error}",
                destination.display()
            )
        })?;
    }
    fs::rename(&temporary, destination).map_err(|error| {
        format!(
            "failed to promote '{}' to '{}': {error}",
            temporary.display(),
            destination.display()
        )
    })
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

#[cfg(test)]
mod tests {
    use super::RuntimeTarget;
    use std::path::Path;

    #[test]
    fn deployment_profiles_resolve_to_distinct_artifact_roots() {
        let target = RuntimeTarget {
            triple: "x86_64-unknown-linux-gnu".to_owned(),
            explicit: true,
        };
        let installation = Path::new("/vapor");

        assert_eq!(
            target.build_output_root(installation, "vapor", "dev"),
            Path::new("/vapor/output/dev/vapor/x86_64-unknown-linux-gnu/dev")
        );
        assert_eq!(
            target.build_output_root(installation, "vapor", "release"),
            Path::new("/vapor/output/release/vapor/x86_64-unknown-linux-gnu/release")
        );
    }
}
