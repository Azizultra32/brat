//! Test 3: Daemon optional
//!
//! Verifies that Brat operations work correctly without gritee-daemon running.
//! All commands should operate directly on the git-based ledger.
//!
//! Scenario:
//! 1. Initialize Brat with --no-daemon (no gritee-daemon)
//! 2. Create convoy
//! 3. Create task
//! 4. Update task status
//! 5. Run doctor --rebuild
//! 6. Verify all operations succeeded

use serde::Deserialize;

use super::helpers::TestRepo;

/// Response data for convoy create.
#[derive(Debug, Deserialize)]
struct ConvoyCreateData {
    convoy_id: String,
    gritee_issue_id: String,
    title: String,
    status: String,
}

/// Response data for task create.
#[derive(Debug, Deserialize)]
struct TaskCreateData {
    task_id: String,
    gritee_issue_id: String,
    convoy_id: String,
    title: String,
    status: String,
}

/// Response data for task update.
#[derive(Debug, Deserialize)]
struct TaskUpdateData {
    task_id: String,
    status: String,
}

/// Response data for doctor rebuild.
#[derive(Debug, Deserialize)]
struct RebuildData {
    sessions_checked: usize,
    sessions_marked_crashed: usize,
    worktrees_cleaned: usize,
    overall_status: String,
}

/// Response data for witness run.
#[derive(Debug, Deserialize)]
struct WitnessRunData {
    iterations: usize,
    total_tasks_found: usize,
    total_sessions_spawned: usize,
    total_errors: usize,
}

/// Response data for status.
#[derive(Debug, Deserialize)]
struct StatusData {
    tasks: StatusTaskSummary,
}

#[derive(Debug, Deserialize)]
struct StatusTaskSummary {
    by_status: StatusTaskCounts,
}

#[derive(Debug, Deserialize)]
struct StatusTaskCounts {
    running: u32,
    blocked: u32,
    needs_review: u32,
}

#[test]
fn test_daemon_optional_basic_operations() {
    // TestRepo::new() already uses --no-daemon --no-tmux
    let repo = TestRepo::new();

    // Create convoy
    let convoy: ConvoyCreateData =
        repo.brat_json(&["convoy", "create", "--title", "No daemon test"]);
    assert!(
        !convoy.convoy_id.is_empty(),
        "convoy_id should not be empty"
    );
    assert_eq!(convoy.title, "No daemon test");
    assert_eq!(convoy.status, "active");
    println!("Created convoy: {}", convoy.convoy_id);

    // Create task linked to convoy
    let task: TaskCreateData = repo.brat_json(&[
        "task",
        "create",
        "--convoy",
        &convoy.convoy_id,
        "--title",
        "Task without daemon",
    ]);
    assert!(!task.task_id.is_empty(), "task_id should not be empty");
    assert_eq!(task.convoy_id, convoy.convoy_id);
    assert_eq!(task.title, "Task without daemon");
    assert_eq!(task.status, "queued");
    println!("Created task: {}", task.task_id);

    // Update task status
    let update: TaskUpdateData =
        repo.brat_json(&["task", "update", &task.task_id, "--status", "running"]);
    assert_eq!(update.task_id, task.task_id);
    assert_eq!(update.status, "running");
    println!("Updated task status to: {}", update.status);

    // Run doctor --rebuild to verify reconciliation works
    let rebuild: RebuildData = repo.brat_json(&["doctor", "--rebuild"]);
    println!(
        "Doctor rebuild: {} sessions checked, status={}",
        rebuild.sessions_checked, rebuild.overall_status
    );

    // Git should still be clean
    repo.assert_git_clean();

    println!("Daemon optional test passed!");
}

#[test]
fn test_daemon_optional_task_transitions() {
    let repo = TestRepo::new();

    // Create convoy and task
    let convoy: ConvoyCreateData =
        repo.brat_json(&["convoy", "create", "--title", "Transition test"]);
    let task: TaskCreateData = repo.brat_json(&[
        "task",
        "create",
        "--convoy",
        &convoy.convoy_id,
        "--title",
        "Transition task",
    ]);

    // Test valid state transitions
    // queued -> running
    let update: TaskUpdateData =
        repo.brat_json(&["task", "update", &task.task_id, "--status", "running"]);
    assert_eq!(update.status, "running");

    // running -> needs-review
    let update: TaskUpdateData =
        repo.brat_json(&["task", "update", &task.task_id, "--status", "needs-review"]);
    // Status is returned as "needsreview" (no hyphen) due to Debug formatting
    assert!(
        update.status == "needsreview" || update.status == "needs-review",
        "expected needsreview or needs-review, got: {}",
        update.status
    );

    // needs-review -> merged (force since we don't have actual merge)
    let update: TaskUpdateData = repo.brat_json(&[
        "task",
        "update",
        &task.task_id,
        "--status",
        "merged",
        "--force",
    ]);
    assert_eq!(update.status, "merged");

    repo.assert_git_clean();
    println!("Task transitions test passed!");
}

#[test]
fn test_witness_recovers_running_task_with_existing_branch() {
    let repo = TestRepo::new();

    let convoy: ConvoyCreateData =
        repo.brat_json(&["convoy", "create", "--title", "Witness recovery"]);
    let task: TaskCreateData = repo.brat_json(&[
        "task",
        "create",
        "--convoy",
        &convoy.convoy_id,
        "--title",
        "Recover branch",
    ]);

    let update: TaskUpdateData =
        repo.brat_json(&["task", "update", &task.task_id, "--status", "running"]);
    assert_eq!(update.status, "running");

    let branch = format!("task-{}", task.task_id);
    repo.git_expect(&["branch", &branch]);

    let witness_output =
        repo.brat_expect(&["witness", "run", "--once", "--engine", "shell", "--json"]);
    let stdout = String::from_utf8_lossy(&witness_output.stdout);
    let witness = serde_json::Deserializer::from_str(&stdout)
        .into_iter::<serde_json::Value>()
        .last()
        .expect("witness JSON output")
        .expect("parse witness output");
    let data: WitnessRunData =
        serde_json::from_value(witness.get("data").expect("witness data").clone())
            .expect("parse witness data");

    assert_eq!(data.iterations, 1);
    assert_eq!(data.total_tasks_found, 1);
    assert_eq!(data.total_sessions_spawned, 0);
    assert_eq!(data.total_errors, 0);

    let status: StatusData = repo.brat_json(&["status"]);
    assert_eq!(status.tasks.by_status.running, 0);
    assert_eq!(status.tasks.by_status.blocked, 1);
    assert_eq!(status.tasks.by_status.needs_review, 0);

    repo.assert_git_clean();
}

#[test]
fn test_witness_recovers_running_task_with_valid_branch_output() {
    let repo = TestRepo::new();

    let convoy: ConvoyCreateData = repo.brat_json(&[
        "convoy",
        "create",
        "--title",
        "Witness recovery valid output",
    ]);
    let task: TaskCreateData = repo.brat_json(&[
        "task",
        "create",
        "--convoy",
        &convoy.convoy_id,
        "--title",
        "Recover branch with commit",
        "--body",
        "Allowed paths:\n- notes/recovery.txt",
    ]);

    let update: TaskUpdateData =
        repo.brat_json(&["task", "update", &task.task_id, "--status", "running"]);
    assert_eq!(update.status, "running");

    let branch = format!("task-{}", task.task_id);
    repo.git_expect(&["checkout", "-b", &branch]);
    std::fs::create_dir_all(repo.path.join("notes")).expect("create notes dir");
    std::fs::write(repo.path.join("notes/recovery.txt"), "recovered\n")
        .expect("write recovered output");
    repo.git_expect(&["add", "notes/recovery.txt"]);
    repo.git_expect(&["commit", "-m", "Recovered task output"]);
    repo.git_expect(&["checkout", "main"]);

    let witness_output =
        repo.brat_expect(&["witness", "run", "--once", "--engine", "shell", "--json"]);
    let stdout = String::from_utf8_lossy(&witness_output.stdout);
    let witness = serde_json::Deserializer::from_str(&stdout)
        .into_iter::<serde_json::Value>()
        .last()
        .expect("witness JSON output")
        .expect("parse witness output");
    let data: WitnessRunData =
        serde_json::from_value(witness.get("data").expect("witness data").clone())
            .expect("parse witness data");

    assert_eq!(data.iterations, 1);
    assert_eq!(data.total_tasks_found, 1);
    assert_eq!(data.total_sessions_spawned, 0);
    assert_eq!(data.total_errors, 0);

    let status: StatusData = repo.brat_json(&["status"]);
    assert_eq!(status.tasks.by_status.running, 0);
    assert_eq!(status.tasks.by_status.blocked, 0);
    assert_eq!(status.tasks.by_status.needs_review, 1);

    repo.assert_git_clean();
}

#[test]
fn test_doctor_check_without_daemon() {
    let repo = TestRepo::new();

    // Doctor check should work without daemon
    let output = repo.brat_expect(&["doctor", "--check"]);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should pass basic checks
    assert!(
        stdout.contains("git_repository") || stdout.contains("PASS"),
        "doctor check should report on git repository"
    );

    repo.assert_git_clean();
    println!("Doctor check test passed!");
}
