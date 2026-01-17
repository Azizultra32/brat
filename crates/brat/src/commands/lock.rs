//! Lock command handler.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::cli::{Cli, LockCommand, LockStatusArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Lock info from Grit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// Resource being locked.
    pub resource: String,
    /// Lock owner (actor ID).
    pub owner: String,
    /// Expiration timestamp (millis since epoch).
    #[serde(default)]
    pub expires_ts: i64,
    /// TTL remaining in milliseconds (computed).
    #[serde(default)]
    pub ttl_remaining_ms: i64,
    /// Whether the lock has expired.
    #[serde(default)]
    pub is_expired: bool,
}

/// Lock conflict info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockConflict {
    /// Resource with conflict.
    pub resource: String,
    /// Actors holding conflicting locks.
    pub holders: Vec<String>,
    /// Summary description.
    pub summary: String,
}

/// Output for lock status command.
#[derive(Debug, Default, Serialize)]
pub struct LockStatusOutput {
    /// Active locks.
    pub locks: Vec<LockInfo>,
    /// Lock conflicts.
    pub conflicts: Vec<LockConflict>,
    /// Total lock count.
    pub total_locks: usize,
    /// Total conflict count.
    pub total_conflicts: usize,
}

/// Grit lock status JSON response envelope.
#[derive(Debug, Deserialize)]
struct GritLockResponse {
    #[serde(default)]
    ok: bool,
    data: Option<GritLockData>,
    error: Option<GritLockError>,
}

#[derive(Debug, Deserialize)]
struct GritLockData {
    #[serde(default)]
    locks: Vec<GritLockEntry>,
}

#[derive(Debug, Deserialize)]
struct GritLockEntry {
    resource: String,
    owner: String,
    #[serde(default)]
    expires_ts: i64,
}

#[derive(Debug, Deserialize)]
struct GritLockError {
    message: String,
}

/// Run the lock command.
pub fn run(cli: &Cli, cmd: &LockCommand) -> Result<(), BratError> {
    match cmd {
        LockCommand::Status(args) => run_status(cli, args),
    }
}

/// Run the lock status command.
fn run_status(cli: &Cli, args: &LockStatusArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_grit_initialized()?;

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Shell out to grit lock status --json
    let grit_output = Command::new("grit")
        .args(["lock", "status", "--json"])
        .current_dir(&ctx.repo_root)
        .output();

    let mut output = LockStatusOutput::default();

    match grit_output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);

            // Try to parse the JSON response
            if let Ok(response) = serde_json::from_str::<GritLockResponse>(&stdout) {
                if let Some(data) = response.data {
                    for entry in data.locks {
                        let ttl_remaining_ms = entry.expires_ts - now_ms;
                        let is_expired = ttl_remaining_ms <= 0;

                        let lock_info = LockInfo {
                            resource: entry.resource,
                            owner: entry.owner,
                            expires_ts: entry.expires_ts,
                            ttl_remaining_ms: ttl_remaining_ms.max(0),
                            is_expired,
                        };

                        // Skip expired locks unless showing all
                        if !is_expired || !args.conflicts_only {
                            output.locks.push(lock_info);
                        }
                    }
                }
            }
        }
        Ok(result) => {
            // Command failed, likely grit doesn't support lock status yet
            let stderr = String::from_utf8_lossy(&result.stderr);
            if !cli.quiet {
                eprintln!("Note: grit lock status not available: {}", stderr.trim());
            }
        }
        Err(e) => {
            if !cli.quiet {
                eprintln!("Note: Could not query locks: {}", e);
            }
        }
    }

    // Detect conflicts (same resource held by multiple owners)
    // For MVP, we just track multiple entries for same resource
    let mut resource_owners: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for lock in &output.locks {
        resource_owners
            .entry(lock.resource.clone())
            .or_default()
            .push(lock.owner.clone());
    }

    for (resource, owners) in resource_owners {
        if owners.len() > 1 {
            output.conflicts.push(LockConflict {
                resource: resource.clone(),
                holders: owners.clone(),
                summary: format!(
                    "Resource '{}' locked by {} actors",
                    resource,
                    owners.len()
                ),
            });
        }
    }

    // Filter to conflicts only if requested
    if args.conflicts_only {
        let conflicting_resources: std::collections::HashSet<_> =
            output.conflicts.iter().map(|c| &c.resource).collect();
        output
            .locks
            .retain(|l| conflicting_resources.contains(&l.resource));
    }

    output.total_locks = output.locks.len();
    output.total_conflicts = output.conflicts.len();

    if !cli.json && !cli.quiet {
        if output.locks.is_empty() {
            print_human(cli, "No active locks");
        } else {
            println!("Active Locks ({}):", output.total_locks);
            for lock in &output.locks {
                let ttl_str = if lock.is_expired {
                    "EXPIRED".to_string()
                } else if lock.ttl_remaining_ms < 60_000 {
                    format!("{}s remaining", lock.ttl_remaining_ms / 1000)
                } else {
                    format!("{}m remaining", lock.ttl_remaining_ms / 60_000)
                };
                println!("  {}  owner: {}  {}", lock.resource, lock.owner, ttl_str);
            }
        }

        if !output.conflicts.is_empty() {
            println!("\nConflicts ({}):", output.total_conflicts);
            for conflict in &output.conflicts {
                println!("  [CONFLICT] {}", conflict.summary);
                for holder in &conflict.holders {
                    println!("    - {}", holder);
                }
            }
        }
    }

    output_success(cli, output);
    Ok(())
}
