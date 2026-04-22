use std::env;
use std::ffi::OsString;
use std::process::Command;

const BRAT_GRITE_BIN: &str = "BRAT_GRITE_BIN";
const CANDIDATES: &[&str] = &["grite", "gritee"];

pub fn resolve_grite_command() -> OsString {
    if let Some(bin) = env::var_os(BRAT_GRITE_BIN) {
        if !bin.is_empty() {
            return bin;
        }
    }

    for candidate in CANDIDATES {
        if command_available(candidate) {
            return OsString::from(candidate);
        }
    }

    OsString::from("grite")
}

pub fn new_grite_command() -> Command {
    Command::new(resolve_grite_command())
}

fn command_available(candidate: &str) -> bool {
    Command::new(candidate).arg("--help").output().is_ok()
}
