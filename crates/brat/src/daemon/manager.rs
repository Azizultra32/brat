//! Daemon manager for bratd.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::error::BratError;

/// Default port for bratd.
pub const DEFAULT_PORT: u16 = 3000;

/// Default idle timeout in seconds (15 minutes).
pub const DEFAULT_IDLE_TIMEOUT: u64 = 900;

/// Status of the daemon.
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    /// Whether the daemon is running.
    pub running: bool,
    /// PID of the daemon (if running).
    pub pid: Option<u32>,
    /// Port the daemon is listening on (if known).
    pub port: u16,
    /// URL to the daemon.
    pub url: String,
}

/// Manages the bratd daemon lifecycle.
pub struct DaemonManager {
    /// Directory for daemon state files (~/.brat or /tmp/brat-{uid}).
    state_dir: PathBuf,
    /// Port the daemon listens on.
    port: u16,
    /// Idle timeout in seconds.
    idle_timeout: u64,
}

impl DaemonManager {
    /// Create a new daemon manager with default settings.
    pub fn new() -> Self {
        Self::with_config(DEFAULT_PORT, DEFAULT_IDLE_TIMEOUT)
    }

    /// Create a new daemon manager with specific port and timeout.
    pub fn with_config(port: u16, idle_timeout: u64) -> Self {
        let state_dir = Self::default_state_dir();
        Self {
            state_dir,
            port,
            idle_timeout,
        }
    }

    /// Get the default state directory.
    fn default_state_dir() -> PathBuf {
        // Try ~/.brat first, fall back to /tmp/brat-{uid}
        if let Some(home) = dirs::home_dir() {
            let brat_dir = home.join(".brat");
            if brat_dir.exists() || fs::create_dir_all(&brat_dir).is_ok() {
                return brat_dir;
            }
        }

        // Fallback to /tmp
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/brat-{}", uid))
    }

    /// Get path to PID file.
    fn pid_file(&self) -> PathBuf {
        self.state_dir.join("bratd.pid")
    }

    /// Get path to log file.
    fn log_file(&self) -> PathBuf {
        self.state_dir.join("bratd.log")
    }

    /// Get the daemon URL.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Get the health check URL.
    fn health_url(&self) -> String {
        format!("{}/api/v1/health", self.url())
    }

    /// Check if the daemon is running by checking PID and health endpoint.
    pub fn is_running(&self) -> bool {
        // First check PID file
        if let Some(pid) = self.read_pid() {
            // Check if process exists
            if Self::process_exists(pid) {
                // Verify with health check
                return self.health_check().is_ok();
            }
        }

        // No PID file or process dead, but maybe daemon is running anyway
        // (e.g., started externally). Try health check.
        self.health_check().is_ok()
    }

    /// Read PID from PID file.
    fn read_pid(&self) -> Option<u32> {
        fs::read_to_string(self.pid_file())
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    /// Check if a process exists.
    fn process_exists(pid: u32) -> bool {
        // Use kill(pid, 0) to check if process exists
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    /// Perform health check against the daemon.
    fn health_check(&self) -> Result<(), BratError> {
        let url = self.health_url();

        // Use a simple blocking HTTP request
        let output = Command::new("curl")
            .args(["-sf", "--max-time", "2", &url])
            .output()
            .map_err(|e| BratError::Other(format!("Failed to run curl: {}", e)))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(BratError::Other("Health check failed".to_string()))
        }
    }

    /// Ensure the daemon is running. Start it if not.
    pub fn ensure_running(&self) -> Result<(), BratError> {
        if self.is_running() {
            return Ok(());
        }

        self.start()
    }

    /// Start the daemon in the background.
    pub fn start(&self) -> Result<(), BratError> {
        // Ensure state directory exists
        fs::create_dir_all(&self.state_dir)
            .map_err(|e| BratError::Other(format!("Failed to create state dir: {}", e)))?;

        // Check if already running
        if self.is_running() {
            return Err(BratError::Other("Daemon is already running".to_string()));
        }

        // Find the brat binary
        let brat_bin = std::env::current_exe()
            .map_err(|e| BratError::Other(format!("Failed to get current exe: {}", e)))?;

        // Open log file
        let log_file = fs::File::create(self.log_file())
            .map_err(|e| BratError::Other(format!("Failed to create log file: {}", e)))?;

        let log_file_err = log_file
            .try_clone()
            .map_err(|e| BratError::Other(format!("Failed to clone log file: {}", e)))?;

        // Start daemon process
        let child = Command::new(&brat_bin)
            .args([
                "api",
                "--port",
                &self.port.to_string(),
                "--idle-timeout",
                &self.idle_timeout.to_string(),
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(log_file_err))
            .spawn()
            .map_err(|e| BratError::Other(format!("Failed to spawn daemon: {}", e)))?;

        let pid = child.id();

        // Write PID file
        fs::write(self.pid_file(), pid.to_string())
            .map_err(|e| BratError::Other(format!("Failed to write PID file: {}", e)))?;

        // Wait for daemon to be ready
        self.wait_for_ready()?;

        Ok(())
    }

    /// Wait for the daemon to be ready (health check passes).
    fn wait_for_ready(&self) -> Result<(), BratError> {
        let max_attempts = 50; // 5 seconds max
        let delay = Duration::from_millis(100);

        for _ in 0..max_attempts {
            if self.health_check().is_ok() {
                return Ok(());
            }
            thread::sleep(delay);
        }

        Err(BratError::Other(
            "Daemon failed to start within timeout".to_string(),
        ))
    }

    /// Stop the daemon gracefully.
    pub fn stop(&self) -> Result<(), BratError> {
        let pid = self.read_pid().ok_or_else(|| {
            BratError::Other("Daemon is not running (no PID file)".to_string())
        })?;

        if !Self::process_exists(pid) {
            // Process already dead, clean up PID file
            let _ = fs::remove_file(self.pid_file());
            return Ok(());
        }

        // Send SIGTERM
        let result = unsafe { libc::kill(pid as i32, libc::SIGTERM) };
        if result != 0 {
            return Err(BratError::Other(format!(
                "Failed to send SIGTERM to PID {}",
                pid
            )));
        }

        // Wait for process to exit
        let max_wait = 50; // 5 seconds
        for _ in 0..max_wait {
            if !Self::process_exists(pid) {
                let _ = fs::remove_file(self.pid_file());
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }

        // Force kill if still running
        let _ = unsafe { libc::kill(pid as i32, libc::SIGKILL) };
        thread::sleep(Duration::from_millis(100));
        let _ = fs::remove_file(self.pid_file());

        Ok(())
    }

    /// Get daemon status.
    pub fn status(&self) -> DaemonStatus {
        let pid = self.read_pid();
        let running = self.is_running();

        DaemonStatus {
            running,
            pid: if running { pid } else { None },
            port: self.port,
            url: self.url(),
        }
    }

    /// Tail the daemon log file.
    pub fn tail_logs(&self, lines: usize) -> Result<Vec<String>, BratError> {
        let log_path = self.log_file();

        if !log_path.exists() {
            return Ok(vec![]);
        }

        let file = fs::File::open(&log_path)
            .map_err(|e| BratError::Other(format!("Failed to open log file: {}", e)))?;

        let reader = BufReader::new(file);
        let all_lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

        // Return last N lines
        let start = all_lines.len().saturating_sub(lines);
        Ok(all_lines[start..].to_vec())
    }
}

impl Default for DaemonManager {
    fn default() -> Self {
        Self::new()
    }
}
