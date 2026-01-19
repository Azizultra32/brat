//! Workflow YAML schema types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A workflow template loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    /// Workflow name (used as identifier).
    pub name: String,

    /// Schema version.
    #[serde(default = "default_version")]
    pub version: u32,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Workflow type: sequential or parallel convoy.
    #[serde(rename = "type")]
    pub workflow_type: WorkflowType,

    /// Input variable definitions.
    #[serde(default)]
    pub inputs: HashMap<String, InputSpec>,

    /// Sequential steps (for `type: workflow`).
    #[serde(default)]
    pub steps: Vec<StepSpec>,

    /// Parallel legs (for `type: convoy`).
    #[serde(default)]
    pub legs: Vec<LegSpec>,

    /// Synthesis step (for `type: convoy`).
    #[serde(default)]
    pub synthesis: Option<SynthesisSpec>,
}

fn default_version() -> u32 {
    1
}

/// Workflow type determines execution model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowType {
    /// Sequential workflow with dependency ordering.
    Workflow,
    /// Parallel convoy with optional synthesis.
    Convoy,
}

/// Input variable specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSpec {
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// Whether this input is required.
    #[serde(default)]
    pub required: bool,

    /// Default value if not provided.
    #[serde(default)]
    pub default: Option<String>,
}

/// A step in a sequential workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepSpec {
    /// Step identifier (unique within workflow).
    pub id: String,

    /// Task title (supports {{variable}} substitution).
    pub title: String,

    /// Task body/description (supports {{variable}} substitution).
    #[serde(default)]
    pub body: String,

    /// IDs of steps this step depends on.
    #[serde(default)]
    pub needs: Vec<String>,

    /// Optional labels to add to the task.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// A leg in a parallel convoy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegSpec {
    /// Leg identifier (unique within workflow).
    pub id: String,

    /// Task title (supports {{variable}} substitution).
    pub title: String,

    /// Task body/description (supports {{variable}} substitution).
    #[serde(default)]
    pub body: String,

    /// Optional labels to add to the task.
    #[serde(default)]
    pub labels: Vec<String>,
}

/// Synthesis step that runs after all legs complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesisSpec {
    /// Task title (supports {{variable}} substitution).
    pub title: String,

    /// Task body/description (supports {{variable}} substitution).
    #[serde(default)]
    pub body: String,

    /// IDs of legs this synthesis depends on (defaults to all legs).
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Optional labels to add to the task.
    #[serde(default)]
    pub labels: Vec<String>,
}

impl WorkflowTemplate {
    /// Validate the workflow template.
    pub fn validate(&self) -> Result<(), String> {
        // Check workflow has appropriate content for its type
        match self.workflow_type {
            WorkflowType::Workflow => {
                if self.steps.is_empty() {
                    return Err("workflow type requires at least one step".to_string());
                }
                if !self.legs.is_empty() {
                    return Err("workflow type should not have legs".to_string());
                }
            }
            WorkflowType::Convoy => {
                if self.legs.is_empty() {
                    return Err("convoy type requires at least one leg".to_string());
                }
                if !self.steps.is_empty() {
                    return Err("convoy type should not have steps".to_string());
                }
            }
        }

        // Check for unique step/leg IDs
        let mut ids = std::collections::HashSet::new();
        for step in &self.steps {
            if !ids.insert(&step.id) {
                return Err(format!("duplicate step id: {}", step.id));
            }
        }
        for leg in &self.legs {
            if !ids.insert(&leg.id) {
                return Err(format!("duplicate leg id: {}", leg.id));
            }
        }

        // Check step dependencies reference valid steps
        for step in &self.steps {
            for dep in &step.needs {
                if !self.steps.iter().any(|s| &s.id == dep) {
                    return Err(format!(
                        "step '{}' depends on unknown step '{}'",
                        step.id, dep
                    ));
                }
                if dep == &step.id {
                    return Err(format!("step '{}' cannot depend on itself", step.id));
                }
            }
        }

        // Check synthesis dependencies reference valid legs
        if let Some(ref synthesis) = self.synthesis {
            for dep in &synthesis.depends_on {
                if !self.legs.iter().any(|l| &l.id == dep) {
                    return Err(format!(
                        "synthesis depends on unknown leg '{}'",
                        dep
                    ));
                }
            }
        }

        Ok(())
    }
}
