//! Workflow parser and loader.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::WorkflowError;
use crate::schema::WorkflowTemplate;

/// Parser for loading workflow templates from YAML files.
pub struct WorkflowParser {
    /// Root directory containing workflows (usually `.brat/workflows/`).
    workflows_dir: PathBuf,
}

impl WorkflowParser {
    /// Create a new parser for the given workflows directory.
    pub fn new(workflows_dir: impl Into<PathBuf>) -> Self {
        Self {
            workflows_dir: workflows_dir.into(),
        }
    }

    /// Create a parser from a repository root.
    ///
    /// Looks for workflows in `.brat/workflows/`.
    pub fn from_repo_root(repo_root: impl AsRef<Path>) -> Self {
        Self::new(repo_root.as_ref().join(".brat").join("workflows"))
    }

    /// Get the workflows directory path.
    pub fn workflows_dir(&self) -> &Path {
        &self.workflows_dir
    }

    /// Check if the workflows directory exists.
    pub fn workflows_dir_exists(&self) -> bool {
        self.workflows_dir.is_dir()
    }

    /// List available workflow names.
    pub fn list_workflows(&self) -> Result<Vec<String>, WorkflowError> {
        if !self.workflows_dir.is_dir() {
            return Ok(Vec::new());
        }

        let mut workflows = Vec::new();
        for entry in fs::read_dir(&self.workflows_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "yaml" || ext == "yml" {
                        if let Some(stem) = path.file_stem() {
                            workflows.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
        workflows.sort();
        Ok(workflows)
    }

    /// Load a workflow by name.
    pub fn load(&self, name: &str) -> Result<WorkflowTemplate, WorkflowError> {
        let path = self.find_workflow_path(name)?;
        self.load_from_path(&path)
    }

    /// Load a workflow from a specific path.
    pub fn load_from_path(&self, path: &Path) -> Result<WorkflowTemplate, WorkflowError> {
        let content = fs::read_to_string(path)?;
        let template: WorkflowTemplate = serde_yaml::from_str(&content)?;

        // Validate the template
        template
            .validate()
            .map_err(WorkflowError::ValidationError)?;

        Ok(template)
    }

    /// Find the path to a workflow file by name.
    fn find_workflow_path(&self, name: &str) -> Result<PathBuf, WorkflowError> {
        // Try .yaml extension first
        let yaml_path = self.workflows_dir.join(format!("{}.yaml", name));
        if yaml_path.is_file() {
            return Ok(yaml_path);
        }

        // Try .yml extension
        let yml_path = self.workflows_dir.join(format!("{}.yml", name));
        if yml_path.is_file() {
            return Ok(yml_path);
        }

        Err(WorkflowError::NotFound(name.to_string()))
    }

    /// Substitute variables in a template string.
    ///
    /// Replaces `{{var}}` with the corresponding value from the vars map.
    pub fn substitute_vars(template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            let pattern = format!("{{{{{}}}}}", key);
            result = result.replace(&pattern, value);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_vars() {
        let vars: HashMap<String, String> = [
            ("name".to_string(), "Alice".to_string()),
            ("count".to_string(), "42".to_string()),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            WorkflowParser::substitute_vars("Hello {{name}}", &vars),
            "Hello Alice"
        );
        assert_eq!(
            WorkflowParser::substitute_vars("Count: {{count}} items", &vars),
            "Count: 42 items"
        );
        assert_eq!(
            WorkflowParser::substitute_vars("{{name}} has {{count}}", &vars),
            "Alice has 42"
        );
        assert_eq!(
            WorkflowParser::substitute_vars("No vars here", &vars),
            "No vars here"
        );
    }
}
