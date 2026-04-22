//! Session log persistence.
//!
//! Session output is cached on disk for local observability and referenced by a
//! stable `sha256:<digest>` contract. Readers verify that digest against the
//! corresponding on-disk log file. Raw Git blob refs, plus legacy
//! `sha256:`-prefixed blob refs, are supported as compatibility read paths.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Write session logs and return the stored reference.
///
/// Logs are written to `<repo_root>/.gritee/logs/<session_id>.log`.
/// The returned reference is a `sha256:<hex>` content digest.
///
/// # Arguments
///
/// * `repo_root` - Repository root directory.
/// * `session_id` - Session identifier.
/// * `lines` - Output lines to write.
///
/// # Returns
///
/// The `sha256:<hex>` content digest.
pub fn write_session_logs(
    repo_root: &Path,
    session_id: &str,
    lines: &[String],
) -> std::io::Result<String> {
    // Ensure .gritee/logs/ directory exists
    let logs_dir = repo_root.join(".gritee").join("logs");
    fs::create_dir_all(&logs_dir)?;

    // Write to .gritee/logs/<session_id>.log
    let log_path = logs_dir.join(format!("{}.log", session_id));
    let content = lines.join("\n");
    fs::write(&log_path, &content)?;

    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();

    Ok(format!("sha256:{:x}", hash))
}

/// Read session logs using the canonical on-disk `sha256:` contract, with
/// raw Git blob refs accepted as a compatibility path.
pub fn read_session_logs(
    repo_root: &Path,
    session_id: &str,
    output_ref: &str,
) -> Result<String, String> {
    if let Some(expected_hash) = output_ref.strip_prefix("sha256:") {
        return match read_legacy_log_file(repo_root, session_id, Some(expected_hash)) {
            Ok(content) => Ok(content),
            Err(file_err) => read_git_blob(repo_root, expected_hash).map_err(|_| file_err),
        };
    }

    read_git_blob(repo_root, output_ref)
}

fn log_path(repo_root: &Path, session_id: &str) -> PathBuf {
    repo_root.join(".gritee").join("logs").join(format!("{}.log", session_id))
}

fn read_git_blob(repo_root: &Path, blob_ref: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["cat-file", "blob", blob_ref])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("failed to read blob: {}", e))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn read_legacy_log_file(
    repo_root: &Path,
    session_id: &str,
    expected_hash: Option<&str>,
) -> Result<String, String> {
    let path = log_path(repo_root, session_id);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;

    if let Some(expected_hash) = expected_hash {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let actual_hash = format!("{:x}", hasher.finalize());
        if actual_hash != expected_hash {
            return Err(format!(
                "log file digest mismatch for {}: expected {}, got {}",
                path.display(),
                expected_hash,
                actual_hash
            ));
        }
    }

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn init_git_repo(path: &Path) {
        let status = Command::new("git")
            .args(["init"])
            .current_dir(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

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

        assert!(hash_ref.starts_with("sha256:"));
        assert_eq!(hash_ref.len(), 7 + 64);

        // Verify log file exists
        let log_path = log_path(&temp_dir, "s-20250117-test");
        assert!(log_path.exists());

        // Verify content
        let content = fs::read_to_string(&log_path).unwrap();
        assert_eq!(content, "Starting task...\nProcessing...\nDone.");

        let read_content = read_session_logs(&temp_dir, "s-20250117-test", &hash_ref).unwrap();
        assert_eq!(read_content, content);

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_write_empty_logs() {
        let temp_dir = std::env::temp_dir().join("brat-logs-test-empty");
        let _ = fs::remove_dir_all(&temp_dir);

        let lines: Vec<String> = vec![];

        let hash_ref = write_session_logs(&temp_dir, "s-20250117-empty", &lines).unwrap();

        assert!(hash_ref.starts_with("sha256:"));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_session_logs_supports_legacy_sha256_refs() {
        let temp_dir = std::env::temp_dir().join("brat-logs-test-legacy");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join(".gritee").join("logs")).unwrap();

        let content = "legacy line 1\nlegacy line 2";
        fs::write(log_path(&temp_dir, "s-legacy"), content).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash_ref = format!("sha256:{:x}", hasher.finalize());

        let read_content = read_session_logs(&temp_dir, "s-legacy", &hash_ref).unwrap();
        assert_eq!(read_content, content);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_session_logs_rejects_sha256_digest_mismatch() {
        let temp_dir = std::env::temp_dir().join("brat-logs-test-mismatch");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(temp_dir.join(".gritee").join("logs")).unwrap();

        fs::write(log_path(&temp_dir, "s-mismatch"), "wrong content").unwrap();
        let result = read_session_logs(&temp_dir, "s-mismatch", "sha256:deadbeef");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_session_logs_supports_git_blob_refs() {
        let temp_dir = std::env::temp_dir().join("brat-logs-test-git-blob");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        init_git_repo(&temp_dir);

        let content = "blob line 1\nblob line 2";
        let output = Command::new("git")
            .args(["hash-object", "-w", "--stdin"])
            .current_dir(&temp_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let mut child = output;
        use std::io::Write as _;
        child.stdin.as_mut().unwrap().write_all(content.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let read_content = read_session_logs(&temp_dir, "unused-session-id", &oid).unwrap();
        assert_eq!(read_content, content);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_read_session_logs_supports_sha256_prefixed_git_blob_refs() {
        let temp_dir = std::env::temp_dir().join("brat-logs-test-prefixed-git-blob");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        init_git_repo(&temp_dir);

        let content = "blob line 1\nblob line 2";
        let output = Command::new("git")
            .args(["hash-object", "-w", "--stdin"])
            .current_dir(&temp_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();

        let mut child = output;
        use std::io::Write as _;
        child.stdin.as_mut().unwrap().write_all(content.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        let oid = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let read_content =
            read_session_logs(&temp_dir, "unused-session-id", &format!("sha256:{}", oid)).unwrap();
        assert_eq!(read_content, content);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
