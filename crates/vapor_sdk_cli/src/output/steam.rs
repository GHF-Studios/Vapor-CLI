use vapor_sdk_core::steam::types::{SteamLoginReport, SteamStatusReport};
use vapor_sdk_core::{CommandSpec, GlobalOptions};

use super::common::{OutputResult, print_spec, status_label};

pub(super) fn print_status(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: SteamStatusReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_status_summary(&report);

    if globals.verbose {
        print_status_details(&report);
        print_spec(spec);
    }

    Ok(())
}

pub(super) fn print_login(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: SteamLoginReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_status_summary(&report.status);
    println!("account: {}", report.account);
    println!("status: {}", status_label(report.exit_status));

    if globals.verbose {
        print_status_details(&report.status);
        println!("steam_args: {}", report.steam_args.join(" "));
        print_spec(spec);
    }

    if report.exit_status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with {}", spec.action, report.exit_status).into())
    }
}

fn print_status_summary(report: &SteamStatusReport) {
    println!("app: {}", report.app_id);
    println!(
        "steamcmd: {}",
        report
            .resolved_steamcmd
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unavailable".to_owned())
    );
    println!("source: {}", report.steamcmd_source.as_str());
}

fn print_status_details(report: &SteamStatusReport) {
    println!("steam_details:");
    println!("  app_id: {}", report.app_id);
    println!("  vapor_home: {}", report.vapor_home.display());
    println!("  steam_home: {}", report.steam_home.display());
    println!(
        "  steamcmd_state_root: {}",
        report.steamcmd_state_root.display()
    );
    println!(
        "  requested_steamcmd: {}",
        report.requested_steamcmd.display()
    );
    println!(
        "  resolved_steamcmd: {}",
        report
            .resolved_steamcmd
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_owned())
    );
    println!("  steamcmd_source: {}", report.steamcmd_source.as_str());
    println!("  auth_model: {}", report.auth_model);
}
