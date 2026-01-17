//! Test 1: Worktree-safe metadata
//!
//! Verifies that Brat operations from the main repository don't leave
//! unstaged changes in git, and that git worktrees remain clean after
//! operations in the main repo.
//!
//! Note: Currently Brat requires initialization per worktree since
//! .brat/ config is stored in the worktree root. This test verifies
//! that git metadata (refs/grit/*) is properly shared across worktrees.

use std::process::Command;

use super::helpers::TestRepo;

#[test]
fn test_worktree_safe_metadata() {
    let repo = TestRepo::new();

    // Verify initial state is clean
    repo.assert_git_clean();

    // Create a worktree
    let wt_path = repo.add_worktree("feature-branch");

    // Run Brat ops from main repo - create a convoy
    let convoy_output = repo.brat_expect(&["convoy", "create", "--title", "Test convoy"]);
    let convoy_stdout = String::from_utf8_lossy(&convoy_output.stdout);

    // Main repo should still be clean after brat operations
    repo.assert_git_clean();

    // Verify worktree's git status is also clean
    // (brat ops in main shouldn't affect worktree's working tree)
    let wt_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&wt_path)
        .output()
        .expect("run git status in worktree");
    let wt_status_str = String::from_utf8_lossy(&wt_status.stdout);
    assert!(
        wt_status_str.trim().is_empty(),
        "worktree git status not clean:\n{}",
        wt_status_str
    );

    // Verify grit refs are accessible from worktree via git
    let wt_refs = Command::new("git")
        .args(["for-each-ref", "refs/grit/"])
        .current_dir(&wt_path)
        .output()
        .expect("run git for-each-ref in worktree");
    assert!(
        wt_refs.status.success(),
        "git for-each-ref failed in worktree"
    );
    let wt_refs_str = String::from_utf8_lossy(&wt_refs.stdout);
    // Should see grit refs from main repo
    assert!(
        !wt_refs_str.is_empty(),
        "worktree should see grit refs from main repo"
    );

    // Final check - main repo still clean
    repo.assert_git_clean();

    println!("Worktree safety test passed!");
    println!("  Convoy created: {}", convoy_stdout.trim());
}

#[test]
fn test_multiple_worktree_operations() {
    let repo = TestRepo::new();

    // Create two worktrees
    let wt1_path = repo.add_worktree("wt1");
    let wt2_path = repo.add_worktree("wt2");

    // Create convoy from main
    repo.brat_expect(&["convoy", "create", "--title", "Multi-worktree test"]);
    repo.assert_git_clean();

    // Verify git status is clean in each worktree
    for (name, path) in [("wt1", &wt1_path), ("wt2", &wt2_path)] {
        // Check git is clean
        let git_status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(path)
            .output()
            .expect(&format!("run git status in {}", name));
        let status_str = String::from_utf8_lossy(&git_status.stdout);
        assert!(
            status_str.trim().is_empty(),
            "{} git status not clean:\n{}",
            name,
            status_str
        );

        // Verify grit refs are accessible
        let refs = Command::new("git")
            .args(["for-each-ref", "refs/grit/"])
            .current_dir(path)
            .output()
            .expect(&format!("run git for-each-ref in {}", name));
        assert!(refs.status.success());
    }

    // Main repo should still be clean
    repo.assert_git_clean();

    println!("Multiple worktree operations test passed!");
}
