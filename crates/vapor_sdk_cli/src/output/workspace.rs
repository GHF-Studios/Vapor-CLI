use vapor_sdk_core::{
    CommandSpec, GlobalOptions, WorkspaceCargoReport, WorkspaceDeployReport, WorkspaceStatusReport,
    WorkspaceSyncReport,
};

use super::common::{OutputResult, present_label, print_spec, status_label};

pub(super) fn print_status(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: WorkspaceStatusReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_status_summary(&report);

    if globals.verbose {
        print_status_details(&report);
        print_spec(spec);
    }

    Ok(())
}

pub(super) fn print_sync(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: WorkspaceSyncReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_status_summary(&report.status);
    println!("changed_paths: {}", report.changed_paths.len());

    if globals.verbose {
        print_status_details(&report.status);
        println!("changed_path_details:");
        if report.changed_paths.is_empty() {
            println!("  none");
        } else {
            for path in &report.changed_paths {
                println!("  {}", path.display());
            }
        }
        print_spec(spec);
    }

    Ok(())
}

pub(super) fn print_cargo(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: WorkspaceCargoReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_cargo_summary(&report);

    if globals.verbose {
        print_cargo_details(&report);
        print_spec(spec);
    }

    if report.status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with {}", spec.action, report.status).into())
    }
}

pub(super) fn print_deploy(
    globals: &GlobalOptions,
    spec: &CommandSpec,
    report: WorkspaceDeployReport,
) -> OutputResult {
    println!("{}", spec.summary);
    print_cargo_summary(&report.build);
    println!("alias: {}", report.alias_executable.display());
    println!("binary: {}", report.deployed_executable.display());
    println!("env: {}", report.activation_script.display());

    if globals.verbose {
        print_cargo_details(&report.build);
        println!("source_executable: {}", report.source_executable.display());
        println!(
            "deployed_executable: {}",
            report.deployed_executable.display()
        );
        println!("alias_executable: {}", report.alias_executable.display());
        println!("activation_script: {}", report.activation_script.display());
        print_spec(spec);
    }

    if report.build.status.success() {
        Ok(())
    } else {
        Err(format!("{} failed with {}", spec.action, report.build.status).into())
    }
}

fn print_cargo_summary(report: &WorkspaceCargoReport) {
    println!(
        "workspace: {} ({})",
        report.workspace_id, report.workspace_name
    );
    println!("command: cargo {}", report.cargo_args.join(" "));
    println!("status: {}", status_label(report.status));
}

fn print_cargo_details(report: &WorkspaceCargoReport) {
    println!("cargo_details:");
    println!(
        "  invocation_directory: {}",
        report.invocation_directory.display()
    );
    println!("  workspace_root: {}", report.workspace_root.display());
    println!("  workspace_name: {}", report.workspace_name);
    println!("  workspace_id: {}", report.workspace_id);
    println!(
        "  working_directory: {}",
        report.working_directory.display()
    );
    println!("  cargo: {}", report.cargo_path.display());
    println!("  rustc: {}", report.rustc_path.display());
    println!("  cargo_home: {}", report.cargo_home.display());
    println!("  cargo_target_dir: {}", report.target_dir.display());
    println!("  cargo_args: {}", report.cargo_args.join(" "));
    println!("  status: {}", report.status);
}

fn print_status_summary(report: &WorkspaceStatusReport) {
    println!(
        "workspace: {} ({})",
        report.workspace_id, report.workspace_name
    );
    println!("root: {}", report.workspace_root.display());
    println!(
        "cargo_manifest: {}",
        present_label(report.cargo_manifest_exists)
    );
    println!("crates_dir: {}", present_label(report.crates_dir_exists));
}

fn print_status_details(report: &WorkspaceStatusReport) {
    println!("workspace_details:");
    println!(
        "  invocation_directory: {}",
        report.invocation_directory.display()
    );
    println!("  workspace_root: {}", report.workspace_root.display());
    println!("  workspace_name: {}", report.workspace_name);
    println!("  workspace_id: {}", report.workspace_id);
    println!("  cargo_manifest_exists: {}", report.cargo_manifest_exists);
    println!("  crates_dir_exists: {}", report.crates_dir_exists);
}
