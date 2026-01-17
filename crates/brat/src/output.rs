use std::io::Write;

use serde::Serialize;

use crate::cli::Cli;
use crate::error::BratError;

/// JSON response envelope (following Grit's pattern).
#[derive(Debug, Serialize)]
pub struct JsonResponse<T: Serialize> {
    pub schema_version: u32,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonError>,
}

/// JSON error details.
#[derive(Debug, Serialize)]
pub struct JsonError {
    pub code: String,
    pub message: String,
}

const SCHEMA_VERSION: u32 = 1;

/// Output a successful result.
pub fn output_success<T: Serialize>(cli: &Cli, data: T) {
    if cli.json {
        let response = JsonResponse {
            schema_version: SCHEMA_VERSION,
            ok: true,
            data: Some(data),
            error: None,
        };
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else if !cli.quiet {
        // For non-JSON output, pretty-print the data as JSON
        // In the future, this could be replaced with custom formatting
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    }
}

/// Output an error.
pub fn output_error(cli: &Cli, err: &BratError) {
    if cli.json {
        let response: JsonResponse<()> = JsonResponse {
            schema_version: SCHEMA_VERSION,
            ok: false,
            data: None,
            error: Some(JsonError {
                code: err.error_code().to_string(),
                message: err.to_string(),
            }),
        };
        eprintln!("{}", serde_json::to_string_pretty(&response).unwrap());
    } else {
        eprintln!("error: {}", err);
    }
}

/// Print a human-readable message (respects --quiet and --json).
pub fn print_human(cli: &Cli, msg: &str) {
    if !cli.json && !cli.quiet {
        println!("{}", msg);
    }
}

/// Output a streaming update (newline-delimited JSON for --json, pretty for human).
///
/// Used for watch/follow modes where multiple updates are emitted over time.
pub fn output_stream<T: Serialize>(cli: &Cli, data: T) {
    if cli.json {
        // NDJSON format for streaming (single line, no envelope for efficiency)
        println!("{}", serde_json::to_string(&data).unwrap());
    } else if !cli.quiet {
        // Human-readable update
        println!("{}", serde_json::to_string_pretty(&data).unwrap());
    }
    // Flush to ensure output is visible immediately
    let _ = std::io::stdout().flush();
}

/// Clear terminal screen for watch mode (human output only).
///
/// Uses ANSI escape codes. In JSON mode, does nothing (for parseable output).
pub fn clear_screen(cli: &Cli) {
    if !cli.json && !cli.quiet {
        // ANSI: clear screen and move cursor to top-left
        print!("\x1B[2J\x1B[1;1H");
        let _ = std::io::stdout().flush();
    }
}
