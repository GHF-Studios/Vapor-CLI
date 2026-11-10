//! CLI-level guardrails for global execution flags.

use std::error::Error;
use std::fmt;

use dialoguer::Confirm;
use owo_colors::{OwoColorize, Stream::Stdout};
use vapor_sdk_core::{
    CommandSpec, GlobalOptions, LeafCommand, PackCommand, PackagepackCommand, RepairCommand,
    SdkCommand, SourceAuthoringCommand, ToolchainCommand, WorkspaceCommand,
};

#[derive(Debug)]
pub(crate) enum SafetyError {
    UnsupportedFlag {
        flag: &'static str,
        action: String,
        reason: &'static str,
    },
    Aborted {
        action: String,
    },
    PromptUnavailable {
        action: String,
    },
}

impl fmt::Display for SafetyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFlag {
                flag,
                action,
                reason,
            } => write!(formatter, "{flag} is not valid for `{action}`: {reason}"),
            Self::Aborted { action } => write!(formatter, "aborted `{action}`"),
            Self::PromptUnavailable { action } => write!(
                formatter,
                "cannot prompt for `{action}` in this terminal; rerun with --yes only if you intend to accept the prompt automatically"
            ),
        }
    }
}

impl Error for SafetyError {}

pub(crate) fn guard(
    globals: &GlobalOptions,
    command: &SdkCommand,
    spec: &CommandSpec,
) -> Result<(), Box<dyn Error>> {
    let policy = SafetyPolicy::for_command(command);

    if globals.force && !policy.force {
        return Err(Box::new(SafetyError::UnsupportedFlag {
            flag: "--force",
            action: spec.action.clone(),
            reason: "this command has no defined force behavior",
        }));
    }

    if globals.strict && !policy.strict {
        return Err(Box::new(SafetyError::UnsupportedFlag {
            flag: "--strict",
            action: spec.action.clone(),
            reason: "this command does not perform validity-tolerant authoring mutation",
        }));
    }

    if globals.keep_unused_versions && !policy.keep_unused_versions {
        return Err(Box::new(SafetyError::UnsupportedFlag {
            flag: "--keep-unused-versions",
            action: spec.action.clone(),
            reason: "this command does not prune installed versions",
        }));
    }

    if (globals.force || policy.confirm) && !globals.yes {
        confirm(globals, spec)?;
    }

    Ok(())
}

fn confirm(globals: &GlobalOptions, spec: &CommandSpec) -> Result<(), Box<dyn Error>> {
    if globals.force {
        eprintln!(
            "{} {}",
            "force requested:".if_supports_color(Stdout, |text| text.yellow()),
            spec.summary
        );
        eprintln!(
            "{}",
            "This may bypass normal safety checks for this command."
                .if_supports_color(Stdout, |text| text.yellow())
        );
    } else {
        eprintln!(
            "{} {}",
            "confirmation required:".if_supports_color(Stdout, |text| text.yellow()),
            spec.summary
        );
    }
    eprintln!(
        "action: {}",
        spec.action
            .as_str()
            .if_supports_color(Stdout, |text| text.bold())
    );

    if !spec.preconditions.is_empty() {
        eprintln!("preconditions:");
        for precondition in spec.preconditions {
            eprintln!("  {precondition}");
        }
    }

    let confirmed = Confirm::new()
        .with_prompt(if globals.force {
            "Proceed with --force?"
        } else {
            "Proceed?"
        })
        .default(false)
        .interact()
        .map_err(|_| {
            Box::new(SafetyError::PromptUnavailable {
                action: spec.action.clone(),
            }) as Box<dyn Error>
        })?;

    if !confirmed {
        return Err(Box::new(SafetyError::Aborted {
            action: spec.action.clone(),
        }));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct SafetyPolicy {
    force: bool,
    strict: bool,
    keep_unused_versions: bool,
    confirm: bool,
}

impl SafetyPolicy {
    fn for_command(command: &SdkCommand) -> Self {
        match command {
            SdkCommand::Repair(RepairCommand::Apply { .. }) => Self {
                force: true,
                strict: false,
                keep_unused_versions: true,
                confirm: true,
            },
            SdkCommand::Toolchain(ToolchainCommand::Install | ToolchainCommand::Repair)
            | SdkCommand::Workspace(WorkspaceCommand::Deploy) => Self {
                force: false,
                strict: false,
                keep_unused_versions: false,
                confirm: true,
            },
            SdkCommand::Packagepack(PackagepackCommand::Compose(_))
            | SdkCommand::Pack {
                command: PackCommand::Compose(_),
                ..
            } => Self {
                force: true,
                strict: true,
                keep_unused_versions: false,
                confirm: false,
            },
            SdkCommand::Packagepack(PackagepackCommand::Author(
                SourceAuthoringCommand::Publish { .. },
            ))
            | SdkCommand::Pack {
                command: PackCommand::Author(SourceAuthoringCommand::Publish { .. }),
                ..
            }
            | SdkCommand::Leaf {
                command: LeafCommand::Author(SourceAuthoringCommand::Publish { .. }),
                ..
            } => Self {
                force: false,
                strict: false,
                keep_unused_versions: false,
                confirm: true,
            },
            _ => Self::none(),
        }
    }

    fn none() -> Self {
        Self {
            force: false,
            strict: false,
            keep_unused_versions: false,
            confirm: false,
        }
    }
}
