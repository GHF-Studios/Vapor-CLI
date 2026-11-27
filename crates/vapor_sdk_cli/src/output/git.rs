use std::collections::BTreeSet;

use vapor_sdk_core::{CommandSpec, GitNativeBuildReport, GlobalOptions};

pub(crate) fn print_build(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: GitNativeBuildReport,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", spec.summary);
    println!("Git: {}", report.git_version);
    println!("cache: {}", if report.cache_hit { "reused" } else { "built" });
    println!("cache key: {}", report.cache_key);
    println!("shadow executable: {}", report.executable.display());
    println!("build inputs: {}", report.runtime_dependencies.iter().map(|(name, version)| format!("{name} {version}")).collect::<Vec<_>>().join(", "));
    let runtime_libraries = report.runtime_libraries.values().collect::<BTreeSet<_>>();
    let runtime_images = report
        .runtime_libraries
        .keys()
        .filter_map(|key| key.split_once(':').map(|(path, _)| path))
        .collect::<BTreeSet<_>>();
    println!(
        "runtime closure: {} ELF helper images, {} unique shared-library identities",
        runtime_images.len(),
        runtime_libraries.len()
    );
    if globals.verbose {
        for identity in runtime_libraries {
            println!("  {identity}");
        }
    }
    println!("distribution eligible: {}", report.distribution_eligible);
    println!("note: this host-linked shadow build is not activated under tools/git and cannot be published");
    if globals.verbose {
        super::common::print_spec(spec);
    }
    Ok(())
}
