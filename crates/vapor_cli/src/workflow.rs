//! Cargo workflows routed through the Steam-installed Vapor setup.

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
