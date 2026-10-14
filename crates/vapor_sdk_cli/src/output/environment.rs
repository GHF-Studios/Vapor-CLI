use vapor_sdk_core::{CommandSpec, EnvironmentReport, GlobalOptions};

use super::common::{OutputResult, print_spec};

pub(super) fn print_status(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: EnvironmentReport,
) -> OutputResult {
    println!("{}", spec.summary);
    println!("home: {}", report.vapor_home.display());
    println!("activate: . \"{}\"", report.activation_script.display());
    println!("path_front:");
    for entry in &report.path_entries {
        println!("  {}", entry.display());
    }

    if globals.verbose {
        println!("environment:");
        println!("  VAPOR_HOME={}", report.vapor_home.display());
        println!("  CARGO_HOME={}", report.cargo_home.display());
        println!("  RUSTUP_HOME={}", report.rustup_home.display());
        println!("  VAPOR_STEAM_HOME={}", report.steam_home.display());
        print_spec(spec);
    }

    Ok(())
}
