use vapor_sdk_core::root::types::{RootPackageReport, RootPublishReport};
use vapor_sdk_core::{CommandSpec, GlobalOptions};

use super::common::{OutputResult, print_spec, status_label};

pub(super) fn print_package(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: RootPackageReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_package_summary(&report);

    if globals.verbose {
        print_package_details(&report);
        print_spec(spec);
    }

    Ok(())
}

pub(super) fn print_publish(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: RootPublishReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_package_summary(&report.package);
    println!("steamcmd: {}", report.steamcmd.display());
    println!("account: {}", report.account);

    match report.status {
        Some(status) => {
            println!("status: {}", status_label(status));
            if !status.success() {
                return Err(format!("{} failed with {}", spec.action, status).into());
            }
        }
        None => println!("status: planned"),
    }

    if globals.verbose {
        print_package_details(&report.package);
        println!("steam_args: {}", report.steam_args.join(" "));
        println!(
            "set_live: {}",
            report.set_live.as_deref().unwrap_or("manual")
        );
        print_spec(spec);
    }

    Ok(())
}

fn print_package_summary(report: &RootPackageReport) {
    println!("app: {} depot: {}", report.app_id, report.depot_id);
    println!("platform: {}", report.host_triple);
    println!("package: {}", report.package_root.display());
    println!("files: {}", report.copied_files);
    println!(
        "status: {}",
        if report.planned { "planned" } else { "ready" }
    );
}

fn print_package_details(report: &RootPackageReport) {
    println!("package_details:");
    println!("  planned: {}", report.planned);
    println!("  app_id: {}", report.app_id);
    println!("  depot_id: {}", report.depot_id);
    println!("  host_triple: {}", report.host_triple);
    println!("  description: {}", report.description);
    println!(
        "  set_live: {}",
        report.set_live.as_deref().unwrap_or("manual")
    );
    println!("  vapor_home: {}", report.vapor_home.display());
    println!("  package_root: {}", report.package_root.display());
    println!("  content_root: {}", report.content_root.display());
    println!("  scripts_root: {}", report.scripts_root.display());
    println!("  steam_build_root: {}", report.steam_build_root.display());
    println!("  app_build_script: {}", report.app_build_script.display());
    println!("  copied_files: {}", report.copied_files);
    println!("  included_roots:");
    for root in &report.included_roots {
        println!("    {}", root.display());
    }
}
