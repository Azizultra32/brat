//! Session reconciliation for crash recovery.
//!
//! This module provides functions to reconcile the expected session state
//! (from Gritee) with the actual state (from the engine). This is used during
//! harness startup and periodic health sweeps to detect and recover from
//! crashed sessions.

use std::collections::{HashMap, HashSet};

use crate::types::{Session, SessionStatus};
use crate::GriteeClient;
use crate::GriteeError;

/// Action to take for reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconciliationAction {
    /// Session exists in both Grite and engine, states match.
    InSync {
        session_id: String,
    },

    /// Session exists in Grite but not in engine - mark as crashed.
    MarkCrashed {
        session_id: String,
        gritee_issue_id: String,
        task_id: String,
    },

    /// Session exists in engine but not in Grite - orphaned session.
    /// This is unusual and should be logged as a warning.
    Orphaned {
        session_id: String,
    },

    /// Session status in Grite doesn't match engine reality.
    UpdateStatus {
        session_id: String,
        current_status: SessionStatus,
        new_status: SessionStatus,
    },
}

/// Result of reconciliation for a single repo.
#[derive(Debug, Default)]
pub struct ReconciliationResult {
    /// Actions to take.
    pub actions: Vec<ReconciliationAction>,

    /// Number of sessions that are in sync.
    pub in_sync_count: usize,

    /// Number of sessions marked as crashed.
    pub crashed_count: usize,

    /// Number of orphaned sessions.
    pub orphaned_count: usize,

    /// Number of status updates needed.
    pub status_update_count: usize,
}

impl ReconciliationResult {
    /// Returns true if all sessions are in sync (no actions needed).
    pub fn is_clean(&self) -> bool {
        self.crashed_count == 0 && self.orphaned_count == 0 && self.status_update_count == 0
    }
}

/// Session info from the engine (minimal info needed for reconciliation).
#[derive(Debug, Clone)]
pub struct EngineSessionInfo {
    /// Session ID.
    pub session_id: String,

    /// Whether the session process is still alive.
    pub alive: bool,

    /// Exit code if the session has exited.
    pub exit_code: Option<i32>,
}

/// Reconcile Gritee state with engine state.
///
/// This compares the sessions recorded in Grite with the sessions running
/// in the engine and returns a list of actions to bring them in sync.
///
/// # Arguments
///
/// * `gritee_client` - Client for querying Griteee state
/// * `engine_sessions` - Current sessions from the engine
///
/// # Returns
///
/// A `ReconciliationResult` with the list of actions to take.
pub fn reconcile_sessions(
    gritee_client: &GriteeClient,
    engine_sessions: &[EngineSessionInfo],
) -> Result<ReconciliationResult, GriteeError> {
    let mut result = ReconciliationResult::default();

    // Step 1: Get all active sessions from Gritee (not in Exit state)
    let gritee_sessions = gritee_client.session_list(None)?;
    let gritee_active: HashMap<String, Session> = gritee_sessions
        .into_iter()
        .filter(|s| s.status != SessionStatus::Exit)
        .map(|s| (s.session_id.clone(), s))
        .collect();

    // Step 2: Build set of engine session IDs (for potential future use)
    let _engine_ids: HashSet<&str> = engine_sessions
        .iter()
        .map(|s| s.session_id.as_str())
        .collect();

    // Step 3: Check sessions in Grite
    for (session_id, session) in &gritee_active {
        if let Some(engine_info) = engine_sessions.iter().find(|e| e.session_id == *session_id) {
            // Session exists in both - check if alive
            if engine_info.alive {
                // In sync
                result.actions.push(ReconciliationAction::InSync {
                    session_id: session_id.clone(),
                });
                result.in_sync_count += 1;
            } else {
                // Engine says dead but Grite shows active - mark crashed
                result.actions.push(ReconciliationAction::MarkCrashed {
                    session_id: session_id.clone(),
                    gritee_issue_id: session.gritee_issue_id.clone(),
                    task_id: session.task_id.clone(),
                });
                result.crashed_count += 1;
            }
        } else {
            // Session in Grite but not in engine - crashed
            result.actions.push(ReconciliationAction::MarkCrashed {
                session_id: session_id.clone(),
                gritee_issue_id: session.gritee_issue_id.clone(),
                task_id: session.task_id.clone(),
            });
            result.crashed_count += 1;
        }
    }

    // Step 4: Check for orphaned sessions (in engine but not in Grite)
    for engine_info in engine_sessions {
        if !gritee_active.contains_key(&engine_info.session_id) {
            result.actions.push(ReconciliationAction::Orphaned {
                session_id: engine_info.session_id.clone(),
            });
            result.orphaned_count += 1;
        }
    }

    Ok(result)
}

/// Execute reconciliation actions.
///
/// This applies the reconciliation actions to update Gritee state.
///
/// # Arguments
///
/// * `gritee_client` - Client for updating Griteee state
/// * `actions` - Actions to execute
///
/// # Returns
///
/// Number of successful actions, and any errors encountered.
pub fn execute_reconciliation(
    gritee_client: &GriteeClient,
    actions: &[ReconciliationAction],
) -> (usize, Vec<GriteeError>) {
    let mut success_count = 0;
    let mut errors = Vec::new();

    for action in actions {
        match action {
            ReconciliationAction::MarkCrashed { session_id, .. } => {
                match gritee_client.session_exit(session_id, -1, "crash", None) {
                    Ok(()) => success_count += 1,
                    Err(e) => errors.push(e),
                }
            }
            ReconciliationAction::UpdateStatus {
                session_id,
                new_status,
                ..
            } => {
                // Force update since we're reconciling
                match gritee_client.session_update_status_with_options(session_id, *new_status, true)
                {
                    Ok(()) => success_count += 1,
                    Err(e) => errors.push(e),
                }
            }
            ReconciliationAction::InSync { .. } => {
                // No action needed
                success_count += 1;
            }
            ReconciliationAction::Orphaned { session_id } => {
                // Log warning but don't fail - we can't create a Grite record
                // without task context
                eprintln!(
                    "Warning: orphaned session {} found in engine but not in Grite",
                    session_id
                );
                success_count += 1;
            }
        }
    }

    (success_count, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconciliation_result_is_clean() {
        let clean = ReconciliationResult::default();
        assert!(clean.is_clean());

        let not_clean = ReconciliationResult {
            crashed_count: 1,
            ..Default::default()
        };
        assert!(!not_clean.is_clean());
    }

    #[test]
    fn test_engine_session_info() {
        let info = EngineSessionInfo {
            session_id: "s-20250117-test".to_string(),
            alive: true,
            exit_code: None,
        };
        assert!(info.alive);

        let dead_info = EngineSessionInfo {
            session_id: "s-20250117-dead".to_string(),
            alive: false,
            exit_code: Some(1),
        };
        assert!(!dead_info.alive);
        assert_eq!(dead_info.exit_code, Some(1));
    }
}
