use vapor_sdk_core::{
    CommandSpec, GlobalOptions, ToolchainInstallReport, ToolchainInstallState, ToolchainStatus,
};

use super::common::{OutputResult, print_lines, print_spec};

pub(super) fn print_status(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    status: ToolchainStatus,
) -> OutputResult {
    println!("{}", spec.summary);
    println!("toolchain: {}", status.toolchain.identifier());
    println!("host: {}", status.host_triple);
    print_install_state(&status.install_state);
    println!(
        "home: {} ({})",
        status.vapor_home.display(),
        status.vapor_home_source.as_str()
    );

    if globals.verbose {
        println!("toolchain_details:");
        println!("  host_supported: {}", status.host_supported);
        println!("  rustup_source: {}", status.rustup_source.as_str());
        println!("  local_rustup: {}", status.local_rustup_path.display());
        println!(
            "  rustup: {}",
            status
                .rustup_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unavailable".to_owned())
        );
        println!("  rustup_home: {}", status.rustup_home.display());
        println!("  cargo_home: {}", status.cargo_home.display());
        println!("  rust_toolchain_root: {}", status.toolchain_root.display());
        println!("  bootstrap_root: {}", status.bootstrap_root.display());
        println!("  output_root: {}", status.output_root.display());
        println!("  cargo: {}", status.cargo_path.display());
        println!("  rustc: {}", status.rustc_path.display());
        print_lines("  supported_hosts", status.supported_host_triples());
        print_lines("  supported_targets", status.supported_target_triples());
        println!("  required_components:");
        for component in status.toolchain.required_components() {
            println!("    {}", component.as_str());
        }
        println!("  channel: {}", status.toolchain.channel);
        println!(
            "  version: {}",
            status
                .toolchain
                .version
                .as_deref()
                .unwrap_or("not version-pinned")
        );
        println!("  date: {}", status.toolchain.date);
        println!(
            "  {}: override with a portable/dev Vapor root",
            vapor_sdk_core::VAPOR_HOME_ENV
        );
        print_spec(spec);
    }

    Ok(())
}

pub(super) fn print_install(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: ToolchainInstallReport,
) -> OutputResult {
    let plan = &report.plan;

    println!("{}", spec.summary);
    println!("toolchain: {}", plan.status.toolchain.identifier());
    println!("host: {}", plan.status.host_triple);
    println!("rustup: {}", plan.rustup_path.display());
    println!("installed_root: {}", report.installed_root.display());
    println!(
        "status: {}",
        super::common::status_label(report.exit_status)
    );

    if globals.verbose {
        println!("install_details:");
        println!("  vapor_home: {}", plan.status.vapor_home.display());
        println!("  rustup_source: {}", plan.status.rustup_source.as_str());
        println!("  rustup_home: {}", plan.status.rustup_home.display());
        println!("  cargo_home: {}", plan.status.cargo_home.display());
        println!(
            "  rust_toolchain_root: {}",
            plan.status.toolchain_root.display()
        );
        println!("  bootstrap_root: {}", plan.status.bootstrap_root.display());
        println!("  output_root: {}", plan.status.output_root.display());
        println!("  rustup_args: {}", plan.rustup_args.join(" "));
        print_owned_lines("  rustup_components", &plan.components);
        print_owned_lines("  rustup_targets", &plan.targets);
        println!("  install_state: {}", plan.status.install_state.as_str());
        print_lines("  supported_hosts", plan.status.supported_host_triples());
        print_lines(
            "  supported_targets",
            plan.status.supported_target_triples(),
        );
        print_spec(spec);
    }

    Ok(())
}

fn print_install_state(state: &ToolchainInstallState) {
    println!("install_state: {}", state.as_str());
    if let ToolchainInstallState::Broken { reason } = state {
        println!("install_problem: {reason}");
    }
}

fn print_owned_lines(label: &str, lines: &[String]) {
    println!("{label}:");
    for line in lines {
        println!("    {line}");
    }
}
