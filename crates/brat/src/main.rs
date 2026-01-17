mod cli;
mod commands;
mod context;
mod error;
mod output;
mod workflows;

use clap::Parser;

use cli::{Cli, Command};
use error::BratError;
use output::output_error;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = run_command(&cli).await;

    if let Err(err) = result {
        output_error(&cli, &err);
        std::process::exit(err.exit_code());
    }
}

async fn run_command(cli: &Cli) -> Result<(), BratError> {
    match &cli.command {
        Command::Init(args) => commands::init::run(cli, args),
        Command::Status(args) => commands::status::run(cli, args),
        Command::Convoy(cmd) => commands::convoy::run(cli, cmd),
        Command::Task(cmd) => commands::task::run(cli, cmd),
        Command::Witness(cmd) => commands::witness::run(cli, cmd).await,
        Command::Refinery(cmd) => commands::refinery::run(cli, cmd).await,
        Command::Session(cmd) => commands::session::run(cli, cmd),
        Command::Lock(cmd) => commands::lock::run(cli, cmd),
        Command::Doctor(args) => commands::doctor::run(cli, args),
    }
}
