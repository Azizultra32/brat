//! bratd - Brat daemon binary
//!
//! This is the standalone daemon binary that can be started directly.
//! It's equivalent to `brat daemon start --foreground`.

use clap::Parser;

/// Brat daemon - HTTP API server for the brat harness
#[derive(Parser, Debug)]
#[command(name = "bratd", version, about)]
struct Args {
    /// Host to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(long, short = 'p', default_value = "3000")]
    port: u16,

    /// Idle timeout in seconds (0 = no timeout)
    #[arg(long, default_value = "900")]
    idle_timeout: u64,

    /// CORS allowed origin (default: allow all)
    #[arg(long)]
    cors_origin: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let config = brat::api::server::ServerConfig {
        host: args.host,
        port: args.port,
        cors_origin: args.cors_origin,
        idle_timeout_secs: if args.idle_timeout == 0 {
            None
        } else {
            Some(args.idle_timeout)
        },
    };

    if let Err(e) = brat::api::run_server(config).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
