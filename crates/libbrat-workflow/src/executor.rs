//! Workflow executor - creates convoys and tasks from workflow templates.

use std::collections::HashMap;

use chrono::Utc;
use uuid::Uuid;

use libbrat_grit::GritClient;

use crate::error::WorkflowError;
use crate::parser::WorkflowParser;
use crate::schema::{WorkflowTemplate, WorkflowType};

/// Result of executing a workflow.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkflowInstance {
    /// Unique instance ID.
    pub instance_id: String,
    /// Workflow name.
    pub workflow_name: String,
    /// Convoy ID created for this instance.
    pub convoy_id: String,
    /// Task IDs created (in execution order).
    pub task_ids: Vec<String>,
    /// Input variables used.
    pub variables: HashMap<String, String>,
    /// Timestamp when executed.
    pub executed_at: String,
}

/// Executor for running workflow templates.
pub struct WorkflowExecutor {
    /// Grit client for creating convoys/tasks.
    grit: GritClient,
}

impl WorkflowExecutor {
    /// Create a new executor with the given Grit client.
    pub fn new(grit: GritClient) -> Self {
        Self { grit }
    }

    /// Execute a workflow template with the given variables.
    pub fn execute(
        &self,
        template: &WorkflowTemplate,
        vars: HashMap<String, String>,
    ) -> Result<WorkflowInstance, WorkflowError> {
        // Validate required inputs
        for (name, spec) in &template.inputs {
            if spec.required && !vars.contains_key(name) {
                if spec.default.is_none() {
                    return Err(WorkflowError::MissingInput(name.clone()));
                }
            }
        }

        // Build complete variables map with defaults
        let mut complete_vars = HashMap::new();
        for (name, spec) in &template.inputs {
            if let Some(value) = vars.get(name) {
                complete_vars.insert(name.clone(), value.clone());
            } else if let Some(ref default) = spec.default {
                complete_vars.insert(name.clone(), default.clone());
            }
        }

        // Generate instance ID
        let instance_id = format!("wf-{}", Uuid::new_v4().to_string().split('-').next().unwrap());

        // Create convoy
        let convoy_title = WorkflowParser::substitute_vars(
            &format!("[{}] {}", template.name, template.description.as_deref().unwrap_or(&template.name)),
            &complete_vars,
        );
        let convoy_body = format!(
            "Workflow instance: {}\nWorkflow: {}\nVariables: {:?}",
            instance_id, template.name, complete_vars
        );

        let convoy = self.grit.convoy_create(&convoy_title, Some(&convoy_body))?;

        // Create tasks based on workflow type
        let task_ids = match template.workflow_type {
            WorkflowType::Workflow => self.create_sequential_tasks(template, &convoy.convoy_id, &complete_vars, &instance_id)?,
            WorkflowType::Convoy => self.create_parallel_tasks(template, &convoy.convoy_id, &complete_vars, &instance_id)?,
        };

        Ok(WorkflowInstance {
            instance_id,
            workflow_name: template.name.clone(),
            convoy_id: convoy.convoy_id,
            task_ids,
            variables: complete_vars,
            executed_at: Utc::now().to_rfc3339(),
        })
    }

    /// Create tasks for a sequential workflow.
    fn create_sequential_tasks(
        &self,
        template: &WorkflowTemplate,
        convoy_id: &str,
        vars: &HashMap<String, String>,
        instance_id: &str,
    ) -> Result<Vec<String>, WorkflowError> {
        let mut task_ids = Vec::new();
        let mut step_to_task: HashMap<String, String> = HashMap::new();

        // Topological sort for dependency ordering
        let ordered_steps = self.topological_sort_steps(template)?;

        for step in ordered_steps {
            let title = WorkflowParser::substitute_vars(&step.title, vars);
            let mut body = WorkflowParser::substitute_vars(&step.body, vars);

            // Add workflow metadata to body
            body = format!(
                "{}\n\n---\nWorkflow: {}\nInstance: {}\nStep: {}",
                body, template.name, instance_id, step.id
            );

            // Add dependency info if this step has dependencies
            if !step.needs.is_empty() {
                let dep_task_ids: Vec<&str> = step.needs
                    .iter()
                    .filter_map(|dep| step_to_task.get(dep).map(|s| s.as_str()))
                    .collect();
                body = format!("{}\nDepends on: {}", body, dep_task_ids.join(", "));
            }

            let task = self.grit.task_create(convoy_id, &title, Some(&body))?;

            // If this step has unmet dependencies, mark it as blocked
            // For now, all steps start as queued - the witness will handle ordering
            // based on the dependency info in the body

            step_to_task.insert(step.id.clone(), task.task_id.clone());
            task_ids.push(task.task_id);
        }

        Ok(task_ids)
    }

    /// Create tasks for a parallel convoy.
    fn create_parallel_tasks(
        &self,
        template: &WorkflowTemplate,
        convoy_id: &str,
        vars: &HashMap<String, String>,
        instance_id: &str,
    ) -> Result<Vec<String>, WorkflowError> {
        let mut task_ids = Vec::new();

        // Create a task for each leg (all start as queued - parallel execution)
        for leg in &template.legs {
            let title = WorkflowParser::substitute_vars(&leg.title, vars);
            let mut body = WorkflowParser::substitute_vars(&leg.body, vars);

            // Add workflow metadata
            body = format!(
                "{}\n\n---\nWorkflow: {}\nInstance: {}\nLeg: {}",
                body, template.name, instance_id, leg.id
            );

            let task = self.grit.task_create(convoy_id, &title, Some(&body))?;
            task_ids.push(task.task_id);
        }

        // Create synthesis task if defined
        if let Some(ref synthesis) = template.synthesis {
            let title = WorkflowParser::substitute_vars(&synthesis.title, vars);
            let mut body = WorkflowParser::substitute_vars(&synthesis.body, vars);

            // Add workflow metadata
            body = format!(
                "{}\n\n---\nWorkflow: {}\nInstance: {}\nSynthesis: true\nDepends on legs: {}",
                body, template.name, instance_id,
                template.legs.iter().map(|l| l.id.as_str()).collect::<Vec<_>>().join(", ")
            );

            let task = self.grit.task_create(convoy_id, &title, Some(&body))?;

            // Note: Synthesis task starts as queued. The witness workflow will
            // check for "Synthesis: true" in the body metadata and only pick it
            // up when all legs are complete. We can't transition directly to
            // blocked because Grit requires queued -> running -> blocked.

            task_ids.push(task.task_id);
        }

        Ok(task_ids)
    }

    /// Topological sort of workflow steps based on dependencies.
    fn topological_sort_steps<'a>(
        &self,
        template: &'a WorkflowTemplate,
    ) -> Result<Vec<&'a crate::schema::StepSpec>, WorkflowError> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut temp_visited = std::collections::HashSet::new();

        // Build step map for quick lookup
        let step_map: HashMap<&str, &crate::schema::StepSpec> = template
            .steps
            .iter()
            .map(|s| (s.id.as_str(), s))
            .collect();

        // Visit each step
        for step in &template.steps {
            if !visited.contains(&step.id) {
                self.visit_step(
                    &step.id,
                    &step_map,
                    &mut visited,
                    &mut temp_visited,
                    &mut result,
                )?;
            }
        }

        Ok(result)
    }

    /// Helper for topological sort - visits a step and its dependencies.
    fn visit_step<'a>(
        &self,
        step_id: &str,
        step_map: &HashMap<&str, &'a crate::schema::StepSpec>,
        visited: &mut std::collections::HashSet<String>,
        temp_visited: &mut std::collections::HashSet<String>,
        result: &mut Vec<&'a crate::schema::StepSpec>,
    ) -> Result<(), WorkflowError> {
        if temp_visited.contains(step_id) {
            return Err(WorkflowError::CircularDependency);
        }
        if visited.contains(step_id) {
            return Ok(());
        }

        temp_visited.insert(step_id.to_string());

        let step = step_map
            .get(step_id)
            .ok_or_else(|| WorkflowError::UnknownStep(step_id.to_string()))?;

        // Visit dependencies first
        for dep in &step.needs {
            self.visit_step(dep, step_map, visited, temp_visited, result)?;
        }

        temp_visited.remove(step_id);
        visited.insert(step_id.to_string());
        result.push(step);

        Ok(())
    }
}
