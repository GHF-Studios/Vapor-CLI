//! Vapor shell executable entrypoint.

use std::process;

fn main() {
    if let Err(error) = vapor_cli::run() {
        eprintln!("vapor: {error}");
        process::exit(1);
    }
}
