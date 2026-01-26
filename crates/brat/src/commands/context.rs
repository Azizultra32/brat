use serde::Serialize;

use crate::cli::{
    Cli, ContextCommand, ContextIndexArgs, ContextProjectArgs, ContextQueryArgs, ContextSetArgs,
    ContextShowArgs,
};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Output of context index command.
#[derive(Debug, Serialize)]
pub struct ContextIndexOutput {
    /// Number of files indexed.
    pub indexed: u32,
    /// Number of files skipped.
    pub skipped: u32,
    /// Total files processed.
    pub total_files: u32,
}

/// Output of context query command.
#[derive(Debug, Serialize)]
pub struct ContextQueryOutput {
    /// The query that was run.
    pub query: String,
    /// Matching symbols.
    pub matches: Vec<SymbolMatchOutput>,
    /// Total match count.
    pub count: usize,
}

/// A symbol match.
#[derive(Debug, Serialize)]
pub struct SymbolMatchOutput {
    /// Symbol name.
    pub symbol: String,
    /// File path.
    pub path: String,
}

/// Output of context show command.
#[derive(Debug, Serialize)]
pub struct ContextShowOutput {
    /// File path.
    pub path: String,
    /// Programming language.
    pub language: String,
    /// Summary.
    pub summary: String,
    /// Content hash.
    pub content_hash: String,
    /// Symbols.
    pub symbols: Vec<SymbolOutput>,
}

/// A symbol.
#[derive(Debug, Serialize)]
pub struct SymbolOutput {
    /// Symbol name.
    pub name: String,
    /// Symbol kind.
    pub kind: String,
    /// Starting line.
    pub line_start: u32,
    /// Ending line.
    pub line_end: u32,
}

/// Output of context project command (list).
#[derive(Debug, Serialize)]
pub struct ContextProjectListOutput {
    /// Entries.
    pub entries: Vec<ContextProjectEntryOutput>,
    /// Count.
    pub count: usize,
}

/// A project context entry.
#[derive(Debug, Serialize)]
pub struct ContextProjectEntryOutput {
    /// Key.
    pub key: String,
    /// Value.
    pub value: String,
}

/// Output of context project command (single key).
#[derive(Debug, Serialize)]
pub struct ContextProjectGetOutput {
    /// Key.
    pub key: String,
    /// Value.
    pub value: String,
}

/// Output of context set command.
#[derive(Debug, Serialize)]
pub struct ContextSetOutput {
    /// Key.
    pub key: String,
    /// Value.
    pub value: String,
    /// Action.
    pub action: String,
}

/// Run the context command.
pub fn run(cli: &Cli, cmd: &ContextCommand) -> Result<(), BratError> {
    match cmd {
        ContextCommand::Index(args) => run_index(cli, args),
        ContextCommand::Query(args) => run_query(cli, args),
        ContextCommand::Show(args) => run_show(cli, args),
        ContextCommand::Project(args) => run_project(cli, args),
        ContextCommand::Set(args) => run_set(cli, args),
    }
}

/// Run context index command.
fn run_index(cli: &Cli, args: &ContextIndexArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let paths: Vec<&str> = args.path.iter().map(|s| s.as_str()).collect();
    let result = client.context_index(&paths, args.force, args.pattern.as_deref())?;

    let output = ContextIndexOutput {
        indexed: result.indexed,
        skipped: result.skipped,
        total_files: result.total_files,
    };

    if !cli.json {
        print_human(
            cli,
            &format!(
                "Indexed {} files ({} skipped, {} total)",
                result.indexed, result.skipped, result.total_files
            ),
        );
    }

    output_success(cli, output);
    Ok(())
}

/// Run context query command.
fn run_query(cli: &Cli, args: &ContextQueryArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let matches = client.context_query(&args.query)?;

    let output = ContextQueryOutput {
        query: args.query.clone(),
        count: matches.len(),
        matches: matches
            .iter()
            .map(|m| SymbolMatchOutput {
                symbol: m.symbol.clone(),
                path: m.path.clone(),
            })
            .collect(),
    };

    if !cli.json {
        if matches.is_empty() {
            print_human(cli, &format!("No matches for '{}'", args.query));
        } else {
            print_human(cli, &format!("Matches for '{}':", args.query));
            for m in &matches {
                print_human(cli, &format!("  {} ({})", m.symbol, m.path));
            }
        }
    }

    output_success(cli, output);
    Ok(())
}

/// Run context show command.
fn run_show(cli: &Cli, args: &ContextShowArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let file_ctx = client.context_show(&args.path)?;

    let output = ContextShowOutput {
        path: file_ctx.path.clone(),
        language: file_ctx.language.clone(),
        summary: file_ctx.summary.clone(),
        content_hash: file_ctx.content_hash.clone(),
        symbols: file_ctx
            .symbols
            .iter()
            .map(|s| SymbolOutput {
                name: s.name.clone(),
                kind: s.kind.clone(),
                line_start: s.line_start,
                line_end: s.line_end,
            })
            .collect(),
    };

    if !cli.json {
        print_human(cli, &format!("File: {}", file_ctx.path));
        print_human(cli, &format!("Language: {}", file_ctx.language));
        print_human(cli, &format!("Summary: {}", file_ctx.summary));
        print_human(cli, &format!("Content hash: {}", file_ctx.content_hash));
        print_human(cli, &format!("Symbols ({}):", file_ctx.symbols.len()));
        for s in &file_ctx.symbols {
            print_human(
                cli,
                &format!("  {} ({}) lines {}-{}", s.name, s.kind, s.line_start, s.line_end),
            );
        }
    }

    output_success(cli, output);
    Ok(())
}

/// Run context project command.
fn run_project(cli: &Cli, args: &ContextProjectArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();

    if let Some(key) = &args.key {
        // Get single key
        match client.context_project_get(key)? {
            Some(value) => {
                let output = ContextProjectGetOutput {
                    key: key.clone(),
                    value: value.clone(),
                };

                if !cli.json {
                    print_human(cli, &format!("{} = {}", key, value));
                }

                output_success(cli, output);
            }
            None => {
                return Err(BratError::GriteeCommandFailed(format!(
                    "project context key '{}' not found",
                    key
                )));
            }
        }
    } else {
        // List all entries
        let entries = client.context_project_list()?;

        let output = ContextProjectListOutput {
            count: entries.len(),
            entries: entries
                .iter()
                .map(|e| ContextProjectEntryOutput {
                    key: e.key.clone(),
                    value: e.value.clone(),
                })
                .collect(),
        };

        if !cli.json {
            if entries.is_empty() {
                print_human(cli, "No project context entries");
            } else {
                print_human(cli, "Project context entries:");
                for e in &entries {
                    print_human(cli, &format!("  {} = {}", e.key, e.value));
                }
            }
        }

        output_success(cli, output);
    }

    Ok(())
}

/// Run context set command.
fn run_set(cli: &Cli, args: &ContextSetArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    client.context_project_set(&args.key, &args.value)?;

    let output = ContextSetOutput {
        key: args.key.clone(),
        value: args.value.clone(),
        action: "set".to_string(),
    };

    if !cli.json {
        print_human(cli, &format!("Set {} = {}", args.key, args.value));
    }

    output_success(cli, output);
    Ok(())
}
