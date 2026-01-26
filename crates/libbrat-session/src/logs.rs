//! Session log persistence.
//!
//! Writes session output logs to disk and computes hash references
//! for the observability contract.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Write session logs and return the hash reference.
///
/// Logs are written to `<repo_root>/.grite/logs/<session_id>.log`.
/// The hash reference is returned in the format `sha256:<hex>`.
///
/// # Arguments
///
/// * `repo_root` - Repository root directory.
/// * `session_id` - Session identifier.
/// * `lines` - Output lines to write.
///
/// # Returns
///
/// The SHA-256 hash reference in the format `sha256:<hex>`.
pub fn write_session_logs(
    repo_root: &Path,
    session_id: &str,
    lines: &[String],
) -> std::io::Result<String> {
    // Ensure .grite/logs/ directory exists
    let logs_dir = repo_root.join(".grite").join("logs");
    fs::create_dir_all(&logs_dir)?;

    // Write to .grite/logs/<session_id>.log
    let log_path = logs_dir.join(format!("{}.log", session_id));
    let content = lines.join("\n");
    fs::write(&log_path, &content)?;

    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    Ok(format!("sha256:{:x}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_write_session_logs() {
        let temp_dir = std::env::temp_dir().join("brat-logs-test");
        let _ = fs::remove_dir_all(&temp_dir);

        let lines = vec![
            "Starting task...".to_string(),
            "Processing...".to_string(),
            "Done.".to_string(),
        ];

        let hash_ref = write_session_logs(&temp_dir, "s-20250117-test", &lines).unwrap();

        // Verify hash format
        assert!(hash_ref.starts_with("sha256:"));
        assert_eq!(hash_ref.len(), 7 + 64); // "sha256:" + 64 hex chars

        // Verify log file exists
        let log_path = temp_dir.join(".grite").join("logs").join("s-20250117-test.log");
        assert!(log_path.exists());

        // Verify content
        let content = fs::read_to_string(&log_path).unwrap();
        assert_eq!(content, "Starting task...\nProcessing...\nDone.");

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_write_empty_logs() {
        let temp_dir = std::env::temp_dir().join("brat-logs-test-empty");
        let _ = fs::remove_dir_all(&temp_dir);

        let lines: Vec<String> = vec![];

        let hash_ref = write_session_logs(&temp_dir, "s-20250117-empty", &lines).unwrap();

        // Verify hash of empty content
        assert!(hash_ref.starts_with("sha256:"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
