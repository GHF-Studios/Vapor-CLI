use std::collections::BTreeSet;

use vapor_sdk_core::{CommandSpec, GitNativeBuildReport, GitWindowsArchiveReport, GlobalOptions};

pub(crate) fn print_build(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: GitNativeBuildReport,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", spec.summary);
    println!("Git: {}", report.git_version);
    println!(
        "cache: {}",
        if report.cache_hit { "reused" } else { "built" }
    );
    println!("cache key: {}", report.cache_key);
    println!("cached executable: {}", report.executable.display());
    println!("payload archive: {}", report.payload_archive.display());
    println!("payload archive size: {} bytes", report.payload_archive_bytes);
    println!("payload archive SHA-256: {}", report.payload_archive_sha256);
    println!("source bundle: {}", report.source_bundle.display());
    println!("source bundle size: {} bytes", report.source_bundle_bytes);
    println!("source bundle SHA-256: {}", report.source_bundle_sha256);
    println!(
        "build inputs: {}",
        report
            .runtime_dependencies
            .iter()
            .map(|(name, version)| format!("{name} {version}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let runtime_libraries = report.runtime_libraries.values().collect::<BTreeSet<_>>();
    let runtime_images = report
        .runtime_libraries
        .keys()
        .filter_map(|key| key.split_once(':').map(|(path, _)| path))
        .collect::<BTreeSet<_>>();
    let is_static = report
        .runtime_dependencies
        .get("linkage")
        .is_some_and(|value| value == "static musl");
    if is_static {
        println!(
            "runtime closure: {} ELF helper images, statically linked for musl",
            runtime_images.len()
        );
    } else {
        println!(
            "runtime closure: {} ELF helper images, {} unique shared-library identities",
            runtime_images.len(),
            runtime_libraries.len()
        );
    }
    if globals.verbose {
        for identity in runtime_libraries {
            println!("  {identity}");
        }
    }
    println!("distribution eligible: {}", report.distribution_eligible);
    println!(
        "note: this source-built candidate remains in the build cache; it is not activated or publishable until its source and license inventory is complete"
    );
    if globals.verbose {
        super::common::print_spec(spec);
    }
    Ok(())
}

pub(crate) fn print_windows_inventory(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: GitWindowsArchiveReport,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", spec.summary);
    println!("MinGit: {}", report.version);
    println!("archive: {}", report.archive_path.display());
    println!("archive size: {} bytes", report.archive_bytes);
    println!("archive SHA-256: {}", report.archive_sha256);
    println!("inventory: {}", report.inventory_path.display());
    println!("files: {}; expanded bytes: {}", report.file_count, report.expanded_bytes);
    println!("executables: {}; DLLs: {}", report.executables.len(), report.dlls.len());
    let distinct_packages = report.package_records.iter().map(|record| record.package.as_str()).collect::<BTreeSet<_>>();
    println!("package-version records: {}; distinct package names: {}", report.package_records.len(), distinct_packages.len());
    for expected in report.expected_paths {
        println!("{}: {}", expected.path, if expected.present { "present" } else { "absent" });
    }
    if let Some(versions) = report.package_versions {
        println!("package versions:\n{}", versions.trim_end());
    } else {
        println!("package versions: absent");
    }
    if let Some(moved) = report.libexec_moved {
        println!("relocated Git helpers (etc/libexec-moved.txt):\n{}", moved.trim_end());
    } else {
        println!("relocated Git helpers (etc/libexec-moved.txt): absent");
    }
    println!("license/notice paths: {}", report.license_and_notice_paths.len());
    for path in report.license_and_notice_paths {
        println!("  {path}");
    }
    println!("note: cache-only inspection; no files were extracted, activated, or published");
    if globals.verbose { super::common::print_spec(spec); }
    Ok(())
}
