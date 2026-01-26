//! Configuration for session monitoring.

use std::time::Duration;

/// Configuration for session monitoring.
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Interval between health checks.
    ///
    /// Default: 10 seconds.
    pub health_poll_interval: Duration,

    /// Interval between heartbeat updates to Gritee.
    ///
    /// Default: 30 seconds.
    pub heartbeat_interval: Duration,

    /// Timeout for individual health check operations.
    ///
    /// Default: 5 seconds.
    pub health_timeout: Duration,

    /// Maximum consecutive health check failures before marking session dead.
    ///
    /// Default: 3.
    pub max_health_failures: u32,

    /// Whether to automatically clean up worktrees when sessions exit.
    ///
    /// Default: true.
    pub cleanup_worktrees: bool,

    /// Number of output lines to capture on session exit.
    ///
    /// Default: 100.
    pub exit_output_lines: usize,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            health_poll_interval: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(30),
            health_timeout: Duration::from_millis(5000),
            max_health_failures: 3,
            cleanup_worktrees: true,
            exit_output_lines: 100,
        }
    }
}

impl MonitorConfig {
    /// Create a new config with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the health poll interval.
    pub fn health_poll_interval(mut self, interval: Duration) -> Self {
        self.health_poll_interval = interval;
        self
    }

    /// Set the heartbeat interval.
    pub fn heartbeat_interval(mut self, interval: Duration) -> Self {
        self.heartbeat_interval = interval;
        self
    }

    /// Set the health timeout.
    pub fn health_timeout(mut self, timeout: Duration) -> Self {
        self.health_timeout = timeout;
        self
    }

    /// Set the maximum health failures.
    pub fn max_health_failures(mut self, max: u32) -> Self {
        self.max_health_failures = max;
        self
    }

    /// Set whether to cleanup worktrees.
    pub fn cleanup_worktrees(mut self, cleanup: bool) -> Self {
        self.cleanup_worktrees = cleanup;
        self
    }

    /// Set the number of exit output lines to capture.
    pub fn exit_output_lines(mut self, lines: usize) -> Self {
        self.exit_output_lines = lines;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MonitorConfig::default();
        assert_eq!(config.health_poll_interval, Duration::from_secs(10));
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.health_timeout, Duration::from_millis(5000));
        assert_eq!(config.max_health_failures, 3);
        assert!(config.cleanup_worktrees);
        assert_eq!(config.exit_output_lines, 100);
    }

    #[test]
    fn test_builder_pattern() {
        let config = MonitorConfig::new()
            .health_poll_interval(Duration::from_secs(5))
            .heartbeat_interval(Duration::from_secs(15))
            .max_health_failures(5)
            .cleanup_worktrees(false);

        assert_eq!(config.health_poll_interval, Duration::from_secs(5));
        assert_eq!(config.heartbeat_interval, Duration::from_secs(15));
        assert_eq!(config.max_health_failures, 5);
        assert!(!config.cleanup_worktrees);
    }
}
