//! Workflow template system for brat.
//!
//! This crate provides YAML-based workflow templates that can be expanded
//! into Grite convoys and tasks. Workflows are stored in `.brat/workflows/`
//! and can be either:
//!
//! - **Sequential workflows** (`type: workflow`): Steps execute in order based on dependencies
//! - **Parallel convoys** (`type: convoy`): Legs execute in parallel with optional synthesis
//!
//! # Example
//!
//! ```yaml
//! # .brat/workflows/code-review.yaml
//! name: code-review
//! version: 1
//! type: convoy
//!
//! inputs:
//!   pr:
//!     description: "Pull request number"
//!     required: true
//!
//! legs:
//!   - id: correctness
//!     title: "Correctness Review"
//!     body: "Review for logic errors..."
//!
//!   - id: performance
//!     title: "Performance Review"
//!     body: "Review for performance issues..."
//!
//! synthesis:
//!   title: "Review Synthesis"
//!   body: "Combine findings..."
//! ```

mod error;
mod executor;
mod parser;
mod schema;

pub use error::WorkflowError;
pub use executor::{WorkflowExecutor, WorkflowInstance};
pub use parser::WorkflowParser;
pub use schema::{
    InputSpec, LegSpec, StepSpec, SynthesisSpec, WorkflowTemplate, WorkflowType,
};
