//! Shared state for the bratd daemon.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use libbrat_config::BratConfig;
use libbrat_grit::GritClient;
use libbrat_worktree::WorktreeManager;
use tokio::sync::RwLock;

/// Global daemon state shared across all request handlers.
#[derive(Clone)]
pub struct DaemonState {
    /// Registry of known repositories.
    pub repos: Arc<RwLock<HashMap<String, Arc<RepoContext>>>>,
    /// Daemon start time for uptime calculation.
    pub start_time: Instant,
    /// Version string.
    pub version: String,
}

impl DaemonState {
    /// Create new daemon state.
    pub fn new() -> Self {
        Self {
            repos: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Get uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Register a repository.
    pub async fn register_repo(&self, path: PathBuf) -> Result<Arc<RepoContext>, String> {
        // Validate the path is a git repo with brat initialized
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Err(format!("Not a git repository: {:?}", path));
        }

        let brat_dir = path.join(".brat");
        let config_path = brat_dir.join("config.toml");
        if !config_path.exists() {
            return Err(format!("Brat not initialized in: {:?}", path));
        }

        // Load config
        let config = BratConfig::load(&config_path)
            .map_err(|e| format!("Failed to load config: {}", e))?;

        // Create Grit client
        let grit = GritClient::new(&path);

        // Create worktree manager
        let worktree_manager = WorktreeManager::new(
            &path,
            &config.swarm.worktree_root,
            config.swarm.max_polecats,
        );

        let repo_id = path_to_repo_id(&path);
        let context = Arc::new(RepoContext {
            id: repo_id.clone(),
            path: path.clone(),
            grit,
            config,
            worktree_manager: Some(worktree_manager),
        });

        let mut repos = self.repos.write().await;
        repos.insert(repo_id, Arc::clone(&context));

        Ok(context)
    }

    /// Get a repository by ID.
    pub async fn get_repo(&self, repo_id: &str) -> Option<Arc<RepoContext>> {
        let repos = self.repos.read().await;
        repos.get(repo_id).cloned()
    }

    /// Unregister a repository.
    pub async fn unregister_repo(&self, repo_id: &str) -> bool {
        let mut repos = self.repos.write().await;
        repos.remove(repo_id).is_some()
    }

    /// List all registered repositories.
    pub async fn list_repos(&self) -> Vec<Arc<RepoContext>> {
        let repos = self.repos.read().await;
        repos.values().cloned().collect()
    }
}

impl Default for DaemonState {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for a single repository.
pub struct RepoContext {
    /// Repository ID (base64 encoded path or short ID).
    pub id: String,
    /// Path to repository root.
    pub path: PathBuf,
    /// Grit client for this repo.
    pub grit: GritClient,
    /// Brat configuration.
    pub config: BratConfig,
    /// Worktree manager (if available).
    pub worktree_manager: Option<WorktreeManager>,
}

/// Convert a path to a repo ID (base64 encoded).
pub fn path_to_repo_id(path: &PathBuf) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(path.to_string_lossy().as_bytes())
}

/// Convert a repo ID back to a path.
#[allow(dead_code)]
pub fn repo_id_to_path(repo_id: &str) -> Result<PathBuf, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(repo_id)
        .map_err(|e| format!("Invalid repo ID: {}", e))?;
    let path_str = String::from_utf8(bytes)
        .map_err(|e| format!("Invalid UTF-8 in repo ID: {}", e))?;
    Ok(PathBuf::from(path_str))
}
