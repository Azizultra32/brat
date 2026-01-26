//! Test helpers for integration tests.

use std::path::PathBuf;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Get the path to the built brat binary.
fn brat_bin() -> PathBuf {
    // Use the binary built by cargo in target/debug
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // from crates/brat to crates
    path.pop(); // from crates to repo root
    path.push("target");
    path.push("debug");
    path.push("brat");
    path
}

/// A temporary git repository with Brat and Grit initialized.
pub struct TestRepo {
    pub dir: TempDir,
    pub path: PathBuf,
}

impl TestRepo {
    /// Create a new test repository with Brat initialized.
    ///
    /// Initializes:
    /// - Git repository with initial commit
    /// - Grit ledger
    /// - Brat configuration (no daemon, no tmux)
    ///
    /// The repository is created in a subdirectory of the temp dir
    /// to allow worktrees to be created as siblings.
    pub fn new() -> Self {
        let dir = TempDir::new().expect("create temp dir");
        // Create repo in a subdirectory so worktrees can be siblings
        let path = dir.path().join("repo");
        std::fs::create_dir(&path).expect("create repo dir");

        // Initialize git repo
        run_cmd_expect(&path, "git", &["init"]);
        run_cmd_expect(&path, "git", &["config", "user.email", "test@test.com"]);
        run_cmd_expect(&path, "git", &["config", "user.name", "Test User"]);

        // Create initial commit
        std::fs::write(path.join("README.md"), "# Test Repository\n").unwrap();
        run_cmd_expect(&path, "git", &["add", "."]);
        run_cmd_expect(&path, "git", &["commit", "-m", "Initial commit"]);

        // Initialize grite (no daemon, no agents.md for clean testing)
        run_cmd_expect(&path, "grite", &["init", "--no-daemon", "--no-agents-md"]);

        // Initialize brat (no daemon, no tmux, no agents.md for isolated testing)
        run_cmd_expect(&path, brat_bin().to_str().unwrap(), &["init", "--no-daemon", "--no-tmux", "--no-agents-md"]);

        // Add .brat/ and AGENTS.md to gitignore so they don't show as untracked
        std::fs::write(path.join(".gitignore"), ".brat/\nAGENTS.md\n").unwrap();
        run_cmd_expect(&path, "git", &["add", ".gitignore"]);
        run_cmd_expect(&path, "git", &["commit", "-m", "Add .gitignore"]);

        Self { dir, path }
    }

    /// Create a new test repository with only git initialized (no grite/brat).
    pub fn new_git_only() -> Self {
        let dir = TempDir::new().expect("create temp dir");
        // Create repo in a subdirectory so worktrees can be siblings
        let path = dir.path().join("repo");
        std::fs::create_dir(&path).expect("create repo dir");

        // Initialize git repo
        run_cmd_expect(&path, "git", &["init"]);
        run_cmd_expect(&path, "git", &["config", "user.email", "test@test.com"]);
        run_cmd_expect(&path, "git", &["config", "user.name", "Test User"]);

        // Create initial commit
        std::fs::write(path.join("README.md"), "# Test Repository\n").unwrap();
        run_cmd_expect(&path, "git", &["add", "."]);
        run_cmd_expect(&path, "git", &["commit", "-m", "Initial commit"]);

        Self { dir, path }
    }

    /// Run a brat command and return output.
    ///
    /// Automatically adds `--no-daemon` to avoid daemon auto-start in tests.
    /// Uses the locally built brat binary instead of the one in PATH.
    /// Sets GRITE_NO_DAEMON=1 to ensure grite commands also skip the daemon.
    pub fn brat(&self, args: &[&str]) -> Output {
        let mut full_args = vec!["--no-daemon"];
        full_args.extend(args);
        Command::new(brat_bin())
            .args(&full_args)
            .env("GRITE_NO_DAEMON", "1")
            .current_dir(&self.path)
            .output()
            .expect("run brat")
    }

    /// Run a brat command and assert it succeeds.
    pub fn brat_expect(&self, args: &[&str]) -> Output {
        let output = self.brat(args);
        assert!(
            output.status.success(),
            "brat {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    /// Run a brat command with --json and parse result.
    pub fn brat_json<T: serde::de::DeserializeOwned>(&self, args: &[&str]) -> T {
        let mut full_args = vec!["--json"];
        full_args.extend(args);
        let output = self.brat(&full_args);
        assert!(
            output.status.success(),
            "brat --json {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );

        // Parse the JSON envelope
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).expect("parse JSON response");

        // Extract data from envelope
        assert!(
            json.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
            "brat command not ok: {}",
            stdout
        );

        let data = json.get("data").expect("no data in response");
        serde_json::from_value(data.clone()).expect("parse data")
    }

    /// Run a git command and return output.
    pub fn git(&self, args: &[&str]) -> Output {
        Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("run git")
    }

    /// Run a git command and assert it succeeds.
    pub fn git_expect(&self, args: &[&str]) -> Output {
        let output = self.git(args);
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    /// Run a grite command and return output.
    ///
    /// Automatically adds `--no-daemon` to avoid IPC timeout issues in tests.
    pub fn grite(&self, args: &[&str]) -> Output {
        let mut full_args = vec!["--no-daemon"];
        full_args.extend(args);
        Command::new("grite")
            .args(&full_args)
            .current_dir(&self.path)
            .output()
            .expect("run grite")
    }

    /// Run a grite command and assert it succeeds.
    pub fn grite_expect(&self, args: &[&str]) -> Output {
        let output = self.grite(args);
        assert!(
            output.status.success(),
            "grite {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    /// Assert git status is clean (no modified/untracked files).
    pub fn assert_git_clean(&self) {
        let output = self.git_expect(&["status", "--porcelain"]);
        let status = String::from_utf8_lossy(&output.stdout);
        assert!(
            status.trim().is_empty(),
            "git status not clean:\n{}",
            status
        );
    }

    /// Get git status as a string.
    pub fn git_status(&self) -> String {
        let output = self.git_expect(&["status", "--porcelain"]);
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    /// Create a worktree at the given path with a new branch.
    ///
    /// The worktree is created as a sibling to the main repo, not inside it,
    /// to avoid git status showing it as untracked.
    pub fn add_worktree(&self, branch: &str) -> PathBuf {
        // Create worktree outside the main repo (in parent directory)
        let wt_path = self.path.parent().unwrap().join(format!("wt-{}", branch));
        self.git_expect(&[
            "worktree",
            "add",
            wt_path.to_str().unwrap(),
            "-b",
            branch,
        ]);
        wt_path
    }

    /// Read the .brat/config.toml file.
    pub fn read_config(&self) -> String {
        let config_path = self.path.join(".brat/config.toml");
        std::fs::read_to_string(config_path).unwrap_or_default()
    }

    /// Write to the .brat/config.toml file.
    pub fn write_config(&self, content: &str) {
        let config_path = self.path.join(".brat/config.toml");
        std::fs::write(config_path, content).expect("write config");
    }
}

/// Run a command in a directory and return output.
///
/// Sets GRITE_NO_DAEMON=1 to ensure all grite/brat commands skip the daemon.
pub fn run_cmd(dir: &PathBuf, cmd: &str, args: &[&str]) -> Output {
    Command::new(cmd)
        .args(args)
        .env("GRITE_NO_DAEMON", "1")
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("run {} {:?}: {}", cmd, args, e))
}

/// Run a command and assert it succeeds.
pub fn run_cmd_expect(dir: &PathBuf, cmd: &str, args: &[&str]) -> Output {
    let output = run_cmd(dir, cmd, args);
    assert!(
        output.status.success(),
        "{} {:?} failed: {}",
        cmd,
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
