use vapor_sdk_core::{CommandSpec, GlobalOptions};

use super::common::{OutputResult, print_spec};

pub(super) fn print(globals: &GlobalOptions, spec: &CommandSpec) -> OutputResult {
    println!("{}", spec.summary);
    println!("status: not_implemented");
    println!("action: {}", spec.action);

    if globals.verbose {
        print_spec(spec);
        println!("flags:");
        println!("  yes: {}", globals.yes);
        println!("  force: {}", globals.force);
        println!("  strict: {}", globals.strict);
        println!("  keep_unused_versions: {}", globals.keep_unused_versions);
    }

    Ok(())
}
