//! # Luminex Core
//!
//! Core editor logic and state management.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                       Editor                             │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐│
//! │  │  Workspace  │ │   Config    │ │  Command Dispatcher ││
//! │  └─────────────┘ └─────────────┘ └─────────────────────┘│
//! │         │                                                │
//! │  ┌──────┴──────────────────────────────────┐            │
//! │  │              Document Manager            │            │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐    │            │
//! │  │  │  Doc 1  │ │  Doc 2  │ │  Doc 3  │    │            │
//! │  │  └─────────┘ └─────────┘ └─────────┘    │            │
//! │  └─────────────────────────────────────────┘            │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Learning: Module Organization
//!
//! Rust modules map to files:
//! - `mod foo;` looks for `foo.rs` or `foo/mod.rs`
//! - `pub use` re-exports items for cleaner public APIs

pub mod command;
pub mod config;
pub mod document;
pub mod editor;
pub mod event;
pub mod keymap;
pub mod workspace;

pub use command::{Command, CommandContext, CommandRegistry};
pub use config::Config;
pub use document::{Document, DocumentId};
pub use editor::Editor;
pub use event::{EditorEvent, EventBus};
pub use keymap::{KeyBinding, Keymap};
pub use workspace::Workspace;

/// Result type for core operations
pub type CoreResult<T> = Result<T, CoreError>;

/// Errors that can occur in core operations
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Document not found: {0}")]
    DocumentNotFound(DocumentId),

    #[error("No active document")]
    NoActiveDocument,

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Buffer error: {0}")]
    Buffer(#[from] luminex_buffer::BufferError),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Command not found: {0}")]
    CommandNotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}
