//! # Luminex - A Modern Text Editor
//!
//! A fast, beautiful, and extensible text editor built in Rust.
//!
//! ## Quick Start
//!
//! ```bash
//! # Run the editor
//! cargo run
//!
//! # Run with a file
//! cargo run -- path/to/file.rs
//!
//! # Run with a workspace
//! cargo run -- --workspace path/to/project
//! ```

use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use luminex_ui::{run, Flags};

/// Luminex - A modern text editor built in Rust
#[derive(Parser, Debug)]
#[command(name = "luminex")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File to open
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Workspace directory to open
    #[arg(short, long, value_name = "DIR")]
    workspace: Option<PathBuf>,

    /// Start in read-only mode
    #[arg(short, long)]
    readonly: bool,

    /// Verbose logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging
    let log_level = match args.verbose {
        0 => tracing::Level::WARN,
        1 => tracing::Level::INFO,
        2 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_level(true),
        )
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            log_level,
        ))
        .init();

    tracing::info!("Starting Luminex v{}", env!("CARGO_PKG_VERSION"));

    // Build launch flags
    let flags = Flags {
        file: args.file.map(|p| p.display().to_string()),
        workspace: args.workspace.map(|p| p.display().to_string()),
    };

    // Run the application
    run(flags).map_err(|e| anyhow::anyhow!("Application error: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from(["luminex"]);
        assert!(args.file.is_none());
        assert!(!args.readonly);
    }

    #[test]
    fn test_args_with_file() {
        let args = Args::parse_from(["luminex", "test.rs"]);
        assert_eq!(args.file, Some(PathBuf::from("test.rs")));
    }
}
