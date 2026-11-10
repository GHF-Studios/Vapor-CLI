//! CLI rendering and typed cleanup operations over Vapor's shared storage API.

use crate::command::CleanupCommand;
use std::path::Path;
use vapor_core::{
    VaporInstallation,
    storage::{self, CleanupCandidate, CleanupCost},
};

pub(crate) fn execute(command: CleanupCommand, home: &Path) -> Result<(), String> {
    let installation = VaporInstallation::from_explicit_root(home.to_path_buf())
        .map_err(|error| error.to_string())?;
    match command {
        CleanupCommand::List { json } => {
            let inventory = storage::inspect(&installation)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&inventory)
                        .map_err(|error| format!("could not encode storage inventory: {error}"))?
                );
            } else {
                print_inventory(&inventory);
            }
            Ok(())
        }
        CleanupCommand::Inspect { id } => {
            let inventory = storage::inspect(&installation)?;
            let candidate = inventory
                .candidates
                .into_iter()
                .find(|candidate| candidate.id == id)
                .ok_or_else(|| {
                    "cleanup item not found; refresh with `vapor cleanup list`".to_owned()
                })?;
            print_preview(&candidate);
            println!("No files were changed.");
            println!(
                "To remove it, run: vapor cleanup remove {} --expected-bytes {} --confirm \"DELETE {}\"",
                candidate.id, candidate.bytes, candidate.id
            );
            Ok(())
        }
        CleanupCommand::Remove {
            id,
            expected_bytes,
            confirm,
        } => {
            let result = storage::remove(&installation, &id, expected_bytes, &confirm)?;
            println!(
                "Removed {} · {} logical bytes. Source and other Vapor data were kept; actual disk space reclaimed may differ.",
                result.relative_path,
                format_bytes(result.measured_bytes_before_delete)
            );
            Ok(())
        }
    }
}

fn print_inventory(inventory: &storage::StorageInventory) {
    println!("Vapor Cleanup · installation storage");
    println!("Root: {}", inventory.home.display());
    println!("Total: {}", format_bytes(inventory.total_bytes));
    if inventory.scan_incomplete {
        println!("WARNING: Some files could not be measured; shown totals may be low.");
    }
    println!();
    println!("STORAGE AREAS");
    println!(
        "{:<26} {:>10}  {:<16} WHY IT IS RETAINED",
        "AREA", "LOGICAL", "STATUS"
    );
    for area in &inventory.areas {
        let status = match area.importance {
            vapor_core::storage::StorageImportance::Essential => "KEEP",
            vapor_core::storage::StorageImportance::Mixed => "INSPECT ITEMS",
            vapor_core::storage::StorageImportance::Unclassified => "REVIEW ONLY",
        };
        println!(
            "{:<26} {:>10}  {:<16} {}",
            area.relative_path,
            format_bytes(area.bytes),
            status,
            area.explanation
        );
    }
    println!();
    println!("SELECTABLE CLEANUP ITEMS");
    if inventory.candidates.is_empty() {
        println!("No eligible generated data found.");
    }
    for candidate in &inventory.candidates {
        print_preview(candidate);
    }
    println!(
        "Deletion is limited to listed IDs and requires matching size plus typed confirmation."
    );
}

fn print_preview(candidate: &CleanupCandidate) {
    let cost = match candidate.cost {
        CleanupCost::Rebuild => "SLOWER REBUILD",
        CleanupCost::Redownload => "RE-DOWNLOAD",
        CleanupCost::Recreate => "RECREATE OUTPUT",
    };
    println!(
        "  [{}] {} · {} logical bytes · {}\n      {}\n      {}",
        candidate.id,
        candidate.category,
        format_bytes(candidate.bytes),
        cost,
        candidate.relative_path,
        candidate.consequence
    );
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut amount = bytes as f64;
    let mut unit = 0;
    while amount >= 1024.0 && unit < UNITS.len() - 1 {
        amount /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{amount:.2} {}", UNITS[unit])
    }
}
