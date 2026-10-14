//! SDK command adapter embedded in the single `vapor` terminal process.

mod cli;
mod output;
mod safety;

use clap::Parser;
use std::ffi::OsString;

pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), String> {
    let parsed = cli::Cli::try_parse_from(args).map_err(|error| error.to_string())?;
    let (globals, command) = parsed.into_parts();
    output::print_command(globals, &command).map_err(|error| error.to_string())
}
