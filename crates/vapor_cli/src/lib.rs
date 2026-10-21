#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod app;
mod cross_toolchain;
mod console;
mod diagnostics;
mod git_provider;
mod ide;
mod launch_session;
mod prompt;

pub mod app_local_tools;
pub mod cargo_metadata;
pub mod command;
pub mod documentation;
pub mod metadata;
pub mod state;
pub mod workflow;

pub use app::run;

// Shared provider operations are implemented in Vapor and re-exported here for CLI compatibility.
pub use vapor_client_provider::{content, discovery, distribution, manifest, steam, workspace};
