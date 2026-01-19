//! Cross-platform process management abstractions.
//!
//! This module provides platform-agnostic APIs for process detachment,
//! signal handling, and shell command execution.

use std::process::Command;

/// Configure a command to run as a detached process that survives parent exit.
///
/// On Unix: Creates a new session using `setsid()`.
/// On Windows: Uses `CREATE_NEW_PROCESS_GROUP` and `CREATE_NO_WINDOW` flags.
#[cfg(unix)]
pub fn configure_detached_process(cmd: &mut Command) {
    use std::os::unix::process::CommandExt;
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
}

#[cfg(windows)]
pub fn configure_detached_process(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    // CREATE_NEW_PROCESS_GROUP (0x200) | CREATE_NO_WINDOW (0x08000000)
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    cmd.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
}

/// Send a Unix signal to a process.
///
/// On Unix: Uses the `nix` crate to send the specified signal.
/// On Windows: Only supports termination (maps all signals to TerminateProcess).
#[cfg(unix)]
pub fn send_signal(pid: u32, signal: i32) -> Result<(), String> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    let sig = Signal::try_from(signal).map_err(|_| format!("invalid signal: {}", signal))?;
    kill(Pid::from_raw(pid as i32), sig).map_err(|e| format!("failed to send signal: {}", e))
}

#[cfg(windows)]
pub fn send_signal(pid: u32, _signal: i32) -> Result<(), String> {
    // Windows doesn't have Unix signals.
    // For graceful shutdown, we could try GenerateConsoleCtrlEvent for CTRL_C,
    // but most processes won't handle it properly when detached.
    // Fall back to termination.
    send_term_signal(pid)
}

/// Send a raw signal using libc (for shell engine compatibility).
///
/// On Unix: Uses `libc::kill` directly.
/// On Windows: Terminates the process.
#[cfg(unix)]
pub fn send_raw_signal(pid: u32, signal: i32) {
    unsafe {
        libc::kill(pid as i32, signal);
    }
}

#[cfg(windows)]
pub fn send_raw_signal(pid: u32, _signal: i32) {
    let _ = send_term_signal(pid);
}

/// Send SIGTERM (or equivalent) to gracefully stop a process.
///
/// On Unix: Sends SIGTERM.
/// On Windows: Terminates the process (no true graceful equivalent).
#[cfg(unix)]
pub fn send_term_signal(pid: u32) -> Result<(), String> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
        .map_err(|e| format!("failed to send SIGTERM: {}", e))
}

#[cfg(windows)]
pub fn send_term_signal(pid: u32) -> Result<(), String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            return Err(format!("failed to open process {}", pid));
        }

        let result = TerminateProcess(handle, 1);
        CloseHandle(handle);

        if result == 0 {
            return Err(format!("failed to terminate process {}", pid));
        }
    }
    Ok(())
}

/// Get the shell command and arguments for running commands.
///
/// On Unix: Returns `("bash", &["-l", "-c"])` for login shell.
/// On Windows: Returns `("cmd", &["/C"])`.
#[cfg(unix)]
pub fn get_shell_command() -> (&'static str, &'static [&'static str]) {
    ("bash", &["-l", "-c"])
}

#[cfg(windows)]
pub fn get_shell_command() -> (&'static str, &'static [&'static str]) {
    ("cmd", &["/C"])
}

/// Check if we're on a Unix platform.
#[cfg(unix)]
pub const fn is_unix() -> bool {
    true
}

#[cfg(windows)]
pub const fn is_unix() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_shell_command() {
        let (shell, args) = get_shell_command();
        #[cfg(unix)]
        {
            assert_eq!(shell, "bash");
            assert_eq!(args, &["-l", "-c"]);
        }
        #[cfg(windows)]
        {
            assert_eq!(shell, "cmd");
            assert_eq!(args, &["/C"]);
        }
    }

    #[test]
    fn test_is_unix() {
        #[cfg(unix)]
        assert!(is_unix());
        #[cfg(windows)]
        assert!(!is_unix());
    }
}
