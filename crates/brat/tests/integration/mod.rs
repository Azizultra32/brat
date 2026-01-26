//! Integration tests for Brat.
//!
//! These tests verify end-to-end convoy-like flows work correctly.
//! Based on acceptance tests from docs/acceptance-tests.md.
//!
//! Test coverage:
//! - Test 1: Worktree-safe metadata (worktree_safe)
//! - Test 3: Daemon optional (daemon_optional)
//! - Test 5: Locks (locks)
//! - Test 6: Doctor monotonic rebuild (doctor_rebuild)
//!
//! Skipped:
//! - Test 2: Union merge of WAL (Grite-level test, covered by Grite's own tests)
//! - Test 4: No silent death (requires engine session lifecycle, tested in unit tests)

pub mod helpers;

// Test modules
pub mod worktree_safe;
pub mod daemon_optional;
pub mod locks;
pub mod doctor_rebuild;
