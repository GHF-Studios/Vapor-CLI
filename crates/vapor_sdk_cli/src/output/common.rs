use std::process::ExitStatus;

use vapor_sdk_core::CommandSpec;

pub(super) type OutputResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn status_label(status: ExitStatus) -> String {
    if status.success() {
        "ok".to_owned()
    } else {
        status.to_string()
    }
}

pub(super) fn present_label(present: bool) -> &'static str {
    if present { "yes" } else { "no" }
}

pub(super) fn print_spec(spec: &CommandSpec) {
    println!("diagnostics:");
    println!("  action: {}", spec.action);
    println!("  state_surface: {:?}", spec.surface);
    if !spec.preconditions.is_empty() {
        print_lines("  preconditions", spec.preconditions);
    }
}

pub(super) fn print_lines(label: &str, lines: &[&str]) {
    println!("{label}:");
    if lines.is_empty() {
        println!("    none");
    } else {
        for line in lines {
            println!("    {line}");
        }
    }
}
