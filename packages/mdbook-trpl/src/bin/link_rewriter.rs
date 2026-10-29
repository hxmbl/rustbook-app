use std::io;

use clap::{self, Parser, Subcommand};
use mdbook_preprocessor::Preprocessor;

use mdbook_trpl::LinkRewriter;

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    if let Some(Command::Supports { renderer }) = cli.command {
        return if LinkRewriter.supports_renderer(&renderer).unwrap() {
            Ok(())
        } else {
            Err(format!("Renderer '{renderer}' is unsupported"))
        };
    }

    let (ctx, book) = mdbook_preprocessor::parse_input(io::stdin())
        .map_err(|e| format!("{e}"))?;
    let processed = LinkRewriter.run(&ctx, book).map_err(|e| format!("{e}"))?;
    serde_json::to_writer(io::stdout(), &processed).map_err(|e| format!("{e}"))
}

/// A preprocessor for rewriting external book links to local equivalents.
#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Is the renderer supported?
    ///
    /// All renderers are supported! This is the contract for mdBook.
    Supports { renderer: String },
}
