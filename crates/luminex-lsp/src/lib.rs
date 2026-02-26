//! # Luminex LSP
//!
//! Language Server Protocol client implementation.
//!
//! ## What is LSP?
//!
//! The Language Server Protocol defines a common interface between
//! editors and language servers. A language server provides:
//! - Autocompletion
//! - Go to definition
//! - Find references
//! - Diagnostics (errors, warnings)
//! - Hover information
//! - Code formatting
//!
//! ## Learning: JSON-RPC
//!
//! LSP uses JSON-RPC 2.0 for communication:
//! ```json
//! // Request
//! {"jsonrpc": "2.0", "id": 1, "method": "textDocument/completion", "params": {...}}
//!
//! // Response
//! {"jsonrpc": "2.0", "id": 1, "result": [...]}
//! ```
//!
//! ## Note
//!
//! This is a placeholder implementation. Full LSP support will be added in Phase 3.

use std::collections::HashMap;
use std::path::PathBuf;

// Re-export LSP types
pub use lsp_types;

/// LSP client errors.
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Failed to start server: {0}")]
    StartFailed(String),

    #[error("Communication error: {0}")]
    Communication(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Server exited")]
    ServerExited,

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Configuration for a language server.
#[derive(Debug, Clone)]
pub struct LspConfig {
    pub language_id: String,
    pub command: String,
    pub args: Vec<String>,
}

/// Manager for multiple language servers.
///
/// This is a placeholder - full implementation coming in Phase 3.
pub struct LspManager {
    configs: HashMap<String, LspConfig>,
}

impl LspManager {
    pub fn new() -> Self {
        let mut configs = HashMap::new();

        // Default configurations for common language servers
        configs.insert(
            "rust".to_string(),
            LspConfig {
                language_id: "rust".to_string(),
                command: "rust-analyzer".to_string(),
                args: vec![],
            },
        );

        configs.insert(
            "python".to_string(),
            LspConfig {
                language_id: "python".to_string(),
                command: "pyright-langserver".to_string(),
                args: vec!["--stdio".to_string()],
            },
        );

        configs.insert(
            "typescript".to_string(),
            LspConfig {
                language_id: "typescript".to_string(),
                command: "typescript-language-server".to_string(),
                args: vec!["--stdio".to_string()],
            },
        );

        Self { configs }
    }

    /// Registers a custom language server configuration.
    pub fn register(&mut self, config: LspConfig) {
        self.configs.insert(config.language_id.clone(), config);
    }

    /// Returns the configuration for a language.
    pub fn get_config(&self, language: &str) -> Option<&LspConfig> {
        self.configs.get(language)
    }

    /// Returns all configured languages.
    pub fn languages(&self) -> impl Iterator<Item = &str> {
        self.configs.keys().map(|s| s.as_str())
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}
