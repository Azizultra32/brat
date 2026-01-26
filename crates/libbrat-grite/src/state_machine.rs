//! State machine validation for Brat entity lifecycles.
//!
//! This module provides generic state machine validation for tasks, sessions,
//! convoys, and roles. Transitions are validated against defined rules before
//! being persisted to Gritee.

use std::fmt::{Debug, Display};
use std::hash::Hash;

use crate::types::{SessionStatus, TaskStatus};

/// Error returned when a state transition is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionError<S> {
    pub from: S,
    pub to: S,
    pub reason: String,
}

impl<S: Display> Display for TransitionError<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid transition from '{}' to '{}': {}",
            self.from, self.to, self.reason
        )
    }
}

impl<S: Debug + Display> std::error::Error for TransitionError<S> {}

/// A validated state transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition<S> {
    pub from: S,
    pub to: S,
    pub forced: bool,
}

/// Trait for states that can be validated by a state machine.
pub trait State: Copy + Clone + PartialEq + Eq + Hash + Debug + Display {
    /// Returns true if this is a terminal state (no outgoing transitions allowed).
    fn is_terminal(&self) -> bool;

    /// Returns true if any state can transition to this state.
    fn is_universal_target(&self) -> bool;

    /// Returns the valid states that can be transitioned to from this state.
    fn valid_targets(&self) -> &'static [Self];
}

/// Generic state machine for validating transitions.
#[derive(Debug, Clone)]
pub struct StateMachine<S: State> {
    _marker: std::marker::PhantomData<S>,
}

impl<S: State + 'static> StateMachine<S> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }

    /// Validate a state transition.
    ///
    /// Returns Ok(Transition) if valid, Err(TransitionError) if invalid.
    /// If `force` is true, any transition is allowed (for admin overrides).
    pub fn validate(
        &self,
        from: S,
        to: S,
        force: bool,
    ) -> Result<Transition<S>, TransitionError<S>> {
        // Force flag bypasses all validation
        if force {
            return Ok(Transition {
                from,
                to,
                forced: true,
            });
        }

        // No-op transitions are always valid
        if from == to {
            return Ok(Transition {
                from,
                to,
                forced: false,
            });
        }

        // Cannot transition out of terminal states
        if from.is_terminal() {
            return Err(TransitionError {
                from,
                to,
                reason: format!("'{}' is a terminal state", from),
            });
        }

        // Universal targets (like Dropped) are always reachable
        if to.is_universal_target() {
            return Ok(Transition {
                from,
                to,
                forced: false,
            });
        }

        // Check if target is in valid transitions
        if from.valid_targets().contains(&to) {
            return Ok(Transition {
                from,
                to,
                forced: false,
            });
        }

        Err(TransitionError {
            from,
            to,
            reason: format!(
                "valid targets from '{}' are: {}",
                from,
                format_targets(from.valid_targets())
            ),
        })
    }
}

impl<S: State + 'static> Default for StateMachine<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a list of valid targets for error messages.
fn format_targets<S: Display>(targets: &[S]) -> String {
    if targets.is_empty() {
        "none (terminal state)".to_string()
    } else {
        targets
            .iter()
            .map(|t| format!("'{}'", t))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

// =============================================================================
// TaskStatus State Implementation
// =============================================================================

impl State for TaskStatus {
    fn is_terminal(&self) -> bool {
        matches!(self, TaskStatus::Merged | TaskStatus::Dropped)
    }

    fn is_universal_target(&self) -> bool {
        matches!(self, TaskStatus::Dropped)
    }

    fn valid_targets(&self) -> &'static [Self] {
        match self {
            // queued -> running (session picks up task)
            TaskStatus::Queued => &[TaskStatus::Running],

            // running -> blocked (resource/dependency constraint)
            // running -> needs-review (work complete)
            TaskStatus::Running => &[TaskStatus::Blocked, TaskStatus::NeedsReview],

            // blocked -> running (constraint resolved)
            TaskStatus::Blocked => &[TaskStatus::Running],

            // needs-review -> merged (approved)
            // needs-review -> blocked (merge conflicts or check failures)
            TaskStatus::NeedsReview => &[TaskStatus::Merged, TaskStatus::Blocked],

            // Terminal states have no valid outgoing transitions
            TaskStatus::Merged => &[],
            TaskStatus::Dropped => &[],
        }
    }
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Queued => write!(f, "queued"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::NeedsReview => write!(f, "needs-review"),
            TaskStatus::Merged => write!(f, "merged"),
            TaskStatus::Dropped => write!(f, "dropped"),
        }
    }
}

// =============================================================================
// SessionStatus State Implementation
// =============================================================================

impl State for SessionStatus {
    fn is_terminal(&self) -> bool {
        matches!(self, SessionStatus::Exit)
    }

    fn is_universal_target(&self) -> bool {
        // Exit can be reached from any state (failure, timeout, user stop)
        matches!(self, SessionStatus::Exit)
    }

    fn valid_targets(&self) -> &'static [Self] {
        match self {
            // spawned -> ready (engine health check passes)
            SessionStatus::Spawned => &[SessionStatus::Ready],

            // ready -> running (first task action begins)
            SessionStatus::Ready => &[SessionStatus::Running],

            // running -> handoff (task ready for review/merge)
            SessionStatus::Running => &[SessionStatus::Handoff],

            // handoff -> (exit only via universal target)
            SessionStatus::Handoff => &[],

            // Terminal state
            SessionStatus::Exit => &[],
        }
    }
}

impl Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Spawned => write!(f, "spawned"),
            SessionStatus::Ready => write!(f, "ready"),
            SessionStatus::Running => write!(f, "running"),
            SessionStatus::Handoff => write!(f, "handoff"),
            SessionStatus::Exit => write!(f, "exit"),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // TaskStatus Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_task_transitions() {
        let sm = StateMachine::<TaskStatus>::new();

        // Valid transitions
        assert!(sm.validate(TaskStatus::Queued, TaskStatus::Running, false).is_ok());
        assert!(sm.validate(TaskStatus::Running, TaskStatus::Blocked, false).is_ok());
        assert!(sm.validate(TaskStatus::Running, TaskStatus::NeedsReview, false).is_ok());
        assert!(sm.validate(TaskStatus::Blocked, TaskStatus::Running, false).is_ok());
        assert!(sm.validate(TaskStatus::NeedsReview, TaskStatus::Merged, false).is_ok());
        assert!(sm.validate(TaskStatus::NeedsReview, TaskStatus::Blocked, false).is_ok());
    }

    #[test]
    fn test_invalid_task_transitions() {
        let sm = StateMachine::<TaskStatus>::new();

        // Invalid: cannot skip states
        assert!(sm.validate(TaskStatus::Queued, TaskStatus::NeedsReview, false).is_err());
        assert!(sm.validate(TaskStatus::Queued, TaskStatus::Merged, false).is_err());

        // Invalid: cannot go backward (except blocked -> running)
        assert!(sm.validate(TaskStatus::Running, TaskStatus::Queued, false).is_err());
        assert!(sm.validate(TaskStatus::NeedsReview, TaskStatus::Running, false).is_err());
    }

    #[test]
    fn test_dropped_from_any_state() {
        let sm = StateMachine::<TaskStatus>::new();

        // Dropped is reachable from any non-terminal state
        for status in [
            TaskStatus::Queued,
            TaskStatus::Running,
            TaskStatus::Blocked,
            TaskStatus::NeedsReview,
        ] {
            assert!(sm.validate(status, TaskStatus::Dropped, false).is_ok());
        }
    }

    #[test]
    fn test_terminal_states_cannot_transition() {
        let sm = StateMachine::<TaskStatus>::new();

        // Cannot transition out of Merged
        let err = sm.validate(TaskStatus::Merged, TaskStatus::Running, false).unwrap_err();
        assert!(err.reason.contains("terminal state"));

        // Cannot transition out of Dropped
        let err = sm.validate(TaskStatus::Dropped, TaskStatus::Running, false).unwrap_err();
        assert!(err.reason.contains("terminal state"));
    }

    #[test]
    fn test_force_bypasses_validation() {
        let sm = StateMachine::<TaskStatus>::new();

        // Force allows any transition, even from terminal states
        let result = sm.validate(TaskStatus::Merged, TaskStatus::Running, true);
        assert!(result.is_ok());
        assert!(result.unwrap().forced);
    }

    #[test]
    fn test_noop_transition_always_valid() {
        let sm = StateMachine::<TaskStatus>::new();

        for status in [
            TaskStatus::Queued,
            TaskStatus::Running,
            TaskStatus::Blocked,
            TaskStatus::NeedsReview,
            TaskStatus::Merged,
            TaskStatus::Dropped,
        ] {
            let result = sm.validate(status, status, false);
            assert!(result.is_ok());
            assert!(!result.unwrap().forced);
        }
    }

    // -------------------------------------------------------------------------
    // SessionStatus Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_valid_session_transitions() {
        let sm = StateMachine::<SessionStatus>::new();

        assert!(sm.validate(SessionStatus::Spawned, SessionStatus::Ready, false).is_ok());
        assert!(sm.validate(SessionStatus::Ready, SessionStatus::Running, false).is_ok());
        assert!(sm.validate(SessionStatus::Running, SessionStatus::Handoff, false).is_ok());
    }

    #[test]
    fn test_exit_from_any_session_state() {
        let sm = StateMachine::<SessionStatus>::new();

        // Exit is reachable from any non-terminal state
        for status in [
            SessionStatus::Spawned,
            SessionStatus::Ready,
            SessionStatus::Running,
            SessionStatus::Handoff,
        ] {
            assert!(sm.validate(status, SessionStatus::Exit, false).is_ok());
        }
    }

    #[test]
    fn test_session_terminal_state() {
        let sm = StateMachine::<SessionStatus>::new();

        // Cannot transition out of Exit
        let err = sm.validate(SessionStatus::Exit, SessionStatus::Running, false).unwrap_err();
        assert!(err.reason.contains("terminal state"));
    }

    #[test]
    fn test_invalid_session_transitions() {
        let sm = StateMachine::<SessionStatus>::new();

        // Cannot skip states
        assert!(sm.validate(SessionStatus::Spawned, SessionStatus::Running, false).is_err());
        assert!(sm.validate(SessionStatus::Ready, SessionStatus::Handoff, false).is_err());

        // Cannot go backward
        assert!(sm.validate(SessionStatus::Running, SessionStatus::Ready, false).is_err());
        assert!(sm.validate(SessionStatus::Handoff, SessionStatus::Running, false).is_err());
    }

    #[test]
    fn test_transition_error_display() {
        let sm = StateMachine::<TaskStatus>::new();
        let err = sm.validate(TaskStatus::Queued, TaskStatus::Merged, false).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("queued"));
        assert!(msg.contains("merged"));
        assert!(msg.contains("valid targets"));
    }
}
