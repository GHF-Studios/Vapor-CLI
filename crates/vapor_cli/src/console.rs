use std::io::{self, IsTerminal, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use vapor_client_protocol::{OperationEvent, OperationStatus};
use vapor_console_core::{ConsoleLevel, ConsoleRecord, ConsoleStyleRole};

pub(super) fn print_operation_event(event: &OperationEvent) {
    let Some(message) = event
        .message
        .as_deref()
        .filter(|message| !message.trim().is_empty())
    else {
        return;
    };
    let level = match event.status {
        OperationStatus::Running | OperationStatus::Cancelling => ConsoleLevel::Info,
        OperationStatus::Succeeded => ConsoleLevel::Success,
        OperationStatus::Failed { .. } => ConsoleLevel::Error,
        OperationStatus::Cancelled => ConsoleLevel::Warn,
    };
    let mut record = ConsoleRecord::new(now_unix_ms(), level, "vapor", message)
        .target(event.action.as_str())
        .field("phase", format!("{:?}", event.phase));
    if let Some(progress) = event.progress {
        record = record.field("progress", format!("{:.0}%", progress.clamp(0.0, 1.0) * 100.0));
    }

    let rendered = render_record(&record, io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none());
    if level == ConsoleLevel::Error {
        let _ = writeln!(io::stderr().lock(), "{rendered}");
    } else {
        let _ = writeln!(io::stdout().lock(), "{rendered}");
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

fn render_record(record: &ConsoleRecord, color: bool) -> String {
    let label = record.level.label();
    let target = record.target.as_deref().unwrap_or(record.source.0.as_str());
    let prefix = format!("[{label}] [{target}]");
    let mut line = if color {
        format!("{}{}{} {}", ansi_color(record.level.style_role()), prefix, "\x1b[0m", record.message)
    } else {
        format!("{prefix} {}", record.message)
    };
    for field in &record.fields {
        line.push_str(&format!("  {}={}", field.key, field.value));
    }
    line
}

fn ansi_color(role: ConsoleStyleRole) -> &'static str {
    match role {
        ConsoleStyleRole::Muted | ConsoleStyleRole::Trace | ConsoleStyleRole::Metadata => "\x1b[90m",
        ConsoleStyleRole::Debug => "\x1b[36m",
        ConsoleStyleRole::Normal => "\x1b[97m",
        ConsoleStyleRole::Success => "\x1b[32m",
        ConsoleStyleRole::Warning => "\x1b[33m",
        ConsoleStyleRole::Error => "\x1b[31m",
        ConsoleStyleRole::Input => "\x1b[35m",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_output_uses_shared_severity_and_metadata_roles() {
        let event = OperationEvent {
            revision: Default::default(),
            id: "operation-1".to_owned(),
            composition_id: None,
            action: "composition.install".to_owned(),
            phase: vapor_client_protocol::OperationPhase::Complete,
            status: OperationStatus::Succeeded,
            progress: Some(1.0),
            message: Some("Installed the composition.".to_owned()),
        };
        let record = ConsoleRecord::new(1, ConsoleLevel::Success, "vapor", "Installed the composition.")
            .target(event.action)
            .field("phase", format!("{:?}", event.phase))
            .field("progress", "100%");

        assert_eq!(render_record(&record, false), "[SUCCESS] [composition.install] Installed the composition.  phase=Complete  progress=100%");
        assert!(render_record(&record, true).starts_with("\x1b[32m"));
    }
}
