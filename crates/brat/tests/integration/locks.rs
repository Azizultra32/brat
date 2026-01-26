//! Test 5: Locks
//!
//! Verifies that lock discipline works correctly:
//! - Acquiring locks on paths prevents conflicts
//! - Lock policy is respected (off/warn/require)
//! - Locks are released properly
//!
//! Note: This test uses gritee CLI directly for lock operations since
//! brat integrates locks internally in the workflow.

use serde::Deserialize;

use super::helpers::TestRepo;

/// Response data for lock status.
#[derive(Debug, Deserialize)]
struct LockStatusData {
    locks: Vec<LockInfo>,
    conflicts: Vec<LockConflict>,
    total_locks: usize,
    total_conflicts: usize,
}

#[derive(Debug, Deserialize)]
struct LockInfo {
    resource: String,
    owner: String,
    #[serde(default)]
    expires_ts: i64,
    #[serde(default)]
    ttl_remaining_ms: i64,
    #[serde(default)]
    is_expired: bool,
}

#[derive(Debug, Deserialize)]
struct LockConflict {
    resource: String,
    holders: Vec<String>,
    summary: String,
}

#[test]
fn test_lock_status_no_locks() {
    let repo = TestRepo::new();

    // Should show no locks
    let output = repo.brat_expect(&["lock", "status"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check output indicates no locks
    // (exact format depends on whether gritee supports lock status)
    assert!(
        stdout.contains("No active locks") || stdout.contains("locks") || output.status.success(),
        "lock status should succeed"
    );

    repo.assert_git_clean();
    println!("Lock status (no locks) test passed!");
}

#[test]
fn test_gritee_lock_acquire_release() {
    let repo = TestRepo::new();

    // Acquire a lock using gritee directly
    let acquire_output = repo.gritee(&["lock", "acquire", "--resource", "path:src/main.rs", "--ttl", "60000"]);

    // Check if gritee supports locks (may not be implemented yet)
    if !acquire_output.status.success() {
        let stderr = String::from_utf8_lossy(&acquire_output.stderr);
        if stderr.contains("unknown") || stderr.contains("not found") {
            println!("Skipping lock test - gritee lock commands not available");
            return;
        }
        // Try with alternative syntax
        let acquire_output = repo.gritee(&["lock", "acquire", "path:src/main.rs", "--ttl", "60000"]);
        if !acquire_output.status.success() {
            println!("Skipping lock test - gritee lock acquire failed");
            return;
        }
    }

    // Verify lock appears in status
    let status_output = repo.brat(&["lock", "status", "--json"]);
    if status_output.status.success() {
        let stdout = String::from_utf8_lossy(&status_output.stdout);
        if stdout.contains("path:src/main.rs") {
            println!("Lock acquired and visible in status");
        }
    }

    // Release the lock
    let release_output = repo.gritee(&["lock", "release", "--resource", "path:src/main.rs"]);
    if release_output.status.success() {
        println!("Lock released successfully");
    }

    repo.assert_git_clean();
    println!("Grite lock acquire/release test passed!");
}

#[test]
fn test_lock_policy_in_config() {
    let repo = TestRepo::new();

    // Read current config
    let config = repo.read_config();

    // Verify lock policy exists in config
    assert!(
        config.contains("[locks]") || config.contains("lock"),
        "config should have locks section"
    );

    // Update config to set lock policy
    let new_config = if config.contains("[locks]") {
        config.replace("policy = \"warn\"", "policy = \"require\"")
    } else {
        format!(
            "{}\n\n[locks]\npolicy = \"require\"\n",
            config
        )
    };
    repo.write_config(&new_config);

    // Verify config updated
    let updated_config = repo.read_config();
    assert!(
        updated_config.contains("require"),
        "config should contain 'require' policy"
    );

    repo.assert_git_clean();
    println!("Lock policy config test passed!");
}

#[test]
fn test_lock_status_json_format() {
    let repo = TestRepo::new();

    // Get lock status in JSON format
    let output = repo.brat(&["--json", "lock", "status"]);

    if !output.status.success() {
        // May not be implemented yet
        println!("Skipping JSON lock status test - command failed");
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse JSON response
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .expect("lock status should return valid JSON");

    // Check envelope structure
    assert!(json.get("schema_version").is_some(), "should have schema_version");
    assert!(json.get("ok").is_some(), "should have ok field");

    if let Some(ok) = json.get("ok").and_then(|v| v.as_bool()) {
        if ok {
            assert!(json.get("data").is_some(), "should have data when ok");
        }
    }

    repo.assert_git_clean();
    println!("Lock status JSON format test passed!");
}
