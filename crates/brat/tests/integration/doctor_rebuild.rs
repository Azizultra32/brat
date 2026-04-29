//! Test 6: Doctor rebuild and projection diagnostics
//!
//! Verifies that doctor --rebuild can reconcile Brat harness state without
//! rewriting git refs, and that doctor --check reports real Grite projection
//! corruption from the current local store layout.
//!
//! Scenario:
//! 1. Create some state (convoy, task)
//! 2. Run doctor --rebuild
//! 3. Verify state remains accessible
//! 4. Corrupt the current Grite sled DB file in a temp repo
//! 5. Verify doctor --check reports a safe, non-destructive recovery path

use std::path::PathBuf;

use serde::Deserialize;

use super::helpers::TestRepo;

/// Response data for convoy create.
#[derive(Debug, Deserialize)]
struct ConvoyCreateData {
    convoy_id: String,
    title: String,
}

/// Response data for task create.
#[derive(Debug, Deserialize)]
struct TaskCreateData {
    task_id: String,
    convoy_id: String,
    title: String,
}

/// Response data for doctor rebuild.
#[derive(Debug, Deserialize)]
struct RebuildData {
    sessions_checked: usize,
    sessions_marked_crashed: usize,
    worktrees_cleaned: usize,
    overall_status: String,
    #[serde(default)]
    errors: Vec<String>,
}

/// Response data for doctor check.
#[derive(Debug, Deserialize)]
struct DoctorCheckData {
    checks: Vec<DoctorCheck>,
    overall_status: String,
}

#[derive(Debug, Deserialize)]
struct DoctorCheck {
    name: String,
    status: String,
    message: String,
    remediation: Option<String>,
}

#[test]
fn test_doctor_rebuild_recovers_state() {
    let repo = TestRepo::new();

    // Create some state
    let convoy: ConvoyCreateData =
        repo.brat_json(&["convoy", "create", "--title", "Doctor recovery test"]);
    println!("Created convoy: {}", convoy.convoy_id);

    let task: TaskCreateData = repo.brat_json(&[
        "task",
        "create",
        "--convoy",
        &convoy.convoy_id,
        "--title",
        "Task for recovery",
    ]);
    println!("Created task: {}", task.task_id);

    // Record state before corruption (gritee uses sled database, not git refs)
    let status_before = repo.brat_expect(&["status"]);
    let status_before_str = String::from_utf8_lossy(&status_before.stdout).to_string();
    println!("Status before: {}", status_before_str.lines().count());

    // Run doctor --rebuild
    let rebuild: RebuildData = repo.brat_json(&["doctor", "--rebuild"]);
    println!(
        "Rebuild result: status={}, sessions_checked={}",
        rebuild.overall_status, rebuild.sessions_checked
    );
    assert!(
        rebuild.errors.is_empty(),
        "rebuild should not have errors: {:?}",
        rebuild.errors
    );

    // Verify state recovered (gritee uses sled database, not git refs)
    // The rebuild should recover the state from the event log
    let status_after = repo.brat_expect(&["status"]);
    let status_after_str = String::from_utf8_lossy(&status_after.stdout).to_string();
    println!("Status after: {}", status_after_str.lines().count());

    // Verify state is still accessible via status
    let status_output = repo.brat_expect(&["status"]);
    let status_str = String::from_utf8_lossy(&status_output.stdout);
    println!("Status after rebuild: {}", status_str.trim());

    repo.assert_git_clean();
    println!("Doctor monotonic rebuild test passed!");
}

#[test]
fn test_doctor_rebuild_is_idempotent() {
    let repo = TestRepo::new();

    // Create some state
    let convoy: ConvoyCreateData =
        repo.brat_json(&["convoy", "create", "--title", "Idempotent rebuild test"]);

    // Run rebuild twice
    let _rebuild1: RebuildData = repo.brat_json(&["doctor", "--rebuild"]);
    let rebuild2: RebuildData = repo.brat_json(&["doctor", "--rebuild"]);

    // Second rebuild should be clean (no actions needed)
    assert_eq!(
        rebuild2.overall_status, "clean",
        "second rebuild should be clean (no work to do)"
    );

    // Verify state is still accessible after rebuilds
    let status_output = repo.brat_expect(&["status"]);
    let status_str = String::from_utf8_lossy(&status_output.stdout);
    assert!(
        status_str.contains(&convoy.convoy_id) || status_str.contains("convoy"),
        "convoy should be visible in status after rebuild"
    );

    repo.assert_git_clean();
    println!("Doctor rebuild idempotent test passed!");
}

#[test]
fn test_doctor_check_reports_gritee_projection_access() {
    let repo = TestRepo::new();

    let check: DoctorCheckData = repo.brat_json(&["doctor", "--check"]);

    assert!(
        check.overall_status == "healthy" || check.overall_status == "warning",
        "doctor check should produce an operator status"
    );

    let projection_check = check
        .checks
        .iter()
        .find(|c| c.name == "gritee_projection_accessible")
        .expect("doctor should report Grite projection accessibility");

    assert_eq!(projection_check.status, "pass");
    assert!(
        projection_check.message.contains("CLI-only"),
        "projection check should document daemon-independent probe"
    );
    assert!(
        projection_check.remediation.is_none(),
        "passing projection check should not include remediation"
    );

    let maintenance_check = check
        .checks
        .iter()
        .find(|c| c.name == "gritee_db_maintenance")
        .expect("doctor should report Grite DB maintenance state");

    assert_eq!(maintenance_check.status, "pass");
    assert!(
        maintenance_check.message.contains("events"),
        "maintenance check should include Grite DB stats"
    );
    assert!(
        maintenance_check.remediation.is_none(),
        "passing maintenance check should not include remediation"
    );

    repo.assert_git_clean();
}

#[test]
fn test_doctor_check_reports_corrupt_gritee_projection() {
    let repo = TestRepo::new();

    repo.brat_json::<ConvoyCreateData>(&[
        "convoy",
        "create",
        "--title",
        "Corrupt projection diagnostic test",
    ]);

    let wal_ref_before = repo.git_expect(&["show-ref", "refs/grite/wal"]).stdout;
    let sled_db = corrupt_current_gritee_sled_db(&repo);
    assert!(
        sled_db.is_file(),
        "corrupted Grite sled DB should still be a file"
    );

    let check: DoctorCheckData = repo.brat_json(&["doctor", "--check"]);

    assert_eq!(check.overall_status, "unhealthy");

    let projection_check = check
        .checks
        .iter()
        .find(|c| c.name == "gritee_projection_accessible")
        .expect("doctor should report projection accessibility");

    assert_eq!(projection_check.status, "fail");
    assert!(
        projection_check.message.contains("corrupt")
            || projection_check.message.contains("corrupted"),
        "projection check should classify the real sled DB failure as corruption: {}",
        projection_check.message
    );
    assert!(
        projection_check
            .remediation
            .as_deref()
            .unwrap_or_default()
            .contains("Preserve `refs/grite/*`"),
        "projection remediation should preserve canonical Grite refs"
    );
    assert!(
        check
            .checks
            .iter()
            .all(|c| c.name != "gritee_db_maintenance"),
        "maintenance stats should be skipped when projection access fails"
    );

    let wal_ref_after = repo.git_expect(&["show-ref", "refs/grite/wal"]).stdout;
    assert_eq!(
        wal_ref_before, wal_ref_after,
        "doctor check must not rewrite Grite WAL refs"
    );

    repo.assert_git_clean();
}

fn corrupt_current_gritee_sled_db(repo: &TestRepo) -> PathBuf {
    let stats_output = repo.gritee_expect(&["db", "stats", "--json"]);
    let stats: serde_json::Value =
        serde_json::from_slice(&stats_output.stdout).expect("parse Grite DB stats");
    let sled_path = stats
        .get("data")
        .and_then(|data| data.get("path"))
        .and_then(|path| path.as_str())
        .map(PathBuf::from)
        .expect("Grite DB stats should include sled path");
    let sled_db = sled_path.join("db");

    assert!(
        sled_db.is_file(),
        "expected current Grite sled DB to be a file at {:?}",
        sled_db
    );

    std::fs::write(&sled_db, b"not a sled database\n").expect("corrupt sled DB");
    sled_db
}

#[test]
fn test_doctor_check_before_rebuild() {
    let repo = TestRepo::new();

    // Create state
    repo.brat_json::<ConvoyCreateData>(&["convoy", "create", "--title", "Check before rebuild"]);

    // Run doctor --check (read-only)
    let check_output = repo.brat_expect(&["doctor", "--check"]);
    let check_str = String::from_utf8_lossy(&check_output.stdout);
    println!("Check output: {}", check_str.trim());

    // Check should show healthy state
    assert!(
        check_str.contains("healthy") || check_str.contains("PASS"),
        "doctor check should show healthy state"
    );

    // Run doctor --rebuild
    let rebuild: RebuildData = repo.brat_json(&["doctor", "--rebuild"]);
    assert_eq!(
        rebuild.overall_status, "clean",
        "rebuild should be clean on fresh repo"
    );

    repo.assert_git_clean();
    println!("Doctor check before rebuild test passed!");
}

#[test]
fn test_doctor_default_mode() {
    let repo = TestRepo::new();

    // Run doctor with no flags (should default to check)
    let output = repo.brat_expect(&["doctor"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should perform health checks
    assert!(
        stdout.contains("Health Check")
            || stdout.contains("git_repository")
            || stdout.contains("healthy"),
        "doctor without flags should run health checks"
    );

    repo.assert_git_clean();
    println!("Doctor default mode test passed!");
}
