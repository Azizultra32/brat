//! Workflow implementations for Brat roles.
//!
//! This module contains the core workflow logic for Witness and Refinery roles,
//! as well as the reconciliation workflow for crash recovery.

mod error;
mod locks;
mod reconcile;
mod refinery;
mod witness;

pub use error::WorkflowError;
pub use locks::{LockHelper, LockPolicy};
pub use reconcile::{ReconcileResult, ReconcileWorkflow};
pub use refinery::{RefineryConfig, RefineryLoopResult, RefineryWorkflow};
pub use witness::{WitnessConfig, WitnessLoopResult, WitnessWorkflow};
