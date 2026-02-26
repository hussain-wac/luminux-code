//! Editor configuration.
//!
//! ## Learning: Serde for Serialization
//!
//! Serde is Rust's standard for serialization/deserialization.
//! The `#[derive(Serialize, Deserialize)]` macro generates
//! code to convert structs to/from JSON, TOML, etc.
//!
//! `#[serde(default)]` uses Default::default() for missing fields,
//! making configs backward-compatible.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Main editor configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Editor behavior settings
    pub editor: EditorConfig,

    /// UI appearance settings
    pub ui: UiConfig,

    /// File handling settings
    pub files: FileConfig,

    /// Keyboard settings
    pub keyboard: KeyboardConfig,

    /// Language-specific settings
    #[serde(default)]
    pub languages: HashMap<String, LanguageConfig>,
}

impl Config {
    /// Loads config from the default location.
    pub fn load() -> Self {
        Self::load_from_default_path().unwrap_or_default()
    }

    /// Loads config from a file.
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Loads from the default config path.
    fn load_from_default_path() -> Result<Self, ConfigError> {
        let path = Self::default_path()?;
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Returns the default config file path.
    pub fn default_path() -> Result<PathBuf, ConfigError> {
        let config_dir = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
        Ok(config_dir.join("luminex").join("config.toml"))
    }

    /// Saves the config to the default location.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::default_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Returns config for a specific language.
    pub fn language(&self, lang: &str) -> LanguageConfig {
        self.languages
            .get(lang)
            .cloned()
            .unwrap_or_else(LanguageConfig::default)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            editor: EditorConfig::default(),
            ui: UiConfig::default(),
            files: FileConfig::default(),
            keyboard: KeyboardConfig::default(),
            languages: HashMap::new(),
        }
    }
}

/// Editor behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorConfig {
    /// Tab width in spaces
    pub tab_size: usize,

    /// Use spaces instead of tabs
    pub use_spaces: bool,

    /// Cursor style
    pub cursor_style: CursorStyle,

    /// Enable line wrapping
    pub word_wrap: bool,

    /// Wrap at column (0 = viewport width)
    pub wrap_column: usize,

    /// Auto-indent on enter
    pub auto_indent: bool,

    /// Auto-close brackets and quotes
    pub auto_close: bool,

    /// Enable multiple cursors
    pub multi_cursor: bool,

    /// Scroll past end of file
    pub scroll_past_end: bool,

    /// Lines of context for scroll
    pub scroll_offset: usize,

    /// Enable vim-style modal editing
    pub vim_mode: bool,

    /// Undo history limit
    pub undo_limit: usize,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            use_spaces: true,
            cursor_style: CursorStyle::Line,
            word_wrap: false,
            wrap_column: 0,
            auto_indent: true,
            auto_close: true,
            multi_cursor: true,
            scroll_past_end: true,
            scroll_offset: 3,
            vim_mode: false,
            undo_limit: 1000,
        }
    }
}

/// Cursor visual style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    Line,
    Block,
    Underline,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self::Line
    }
}

/// UI appearance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Color theme name
    pub theme: String,

    /// Font family
    pub font_family: String,

    /// Font size in points
    pub font_size: f32,

    /// Line height multiplier
    pub line_height: f32,

    /// Show line numbers
    pub line_numbers: bool,

    /// Relative line numbers
    pub relative_line_numbers: bool,

    /// Highlight current line
    pub highlight_current_line: bool,

    /// Show indent guides
    pub indent_guides: bool,

    /// Show minimap
    pub minimap: bool,

    /// Minimap width
    pub minimap_width: usize,

    /// Show breadcrumbs
    pub breadcrumbs: bool,

    /// Animation duration in ms (0 to disable)
    pub animation_duration: u32,

    /// Window opacity (0.0 - 1.0)
    pub opacity: f32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_family: "JetBrains Mono".to_string(),
            font_size: 14.0,
            line_height: 1.4,
            line_numbers: true,
            relative_line_numbers: false,
            highlight_current_line: true,
            indent_guides: true,
            minimap: true,
            minimap_width: 100,
            breadcrumbs: true,
            animation_duration: 150,
            opacity: 1.0,
        }
    }
}

/// File handling configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FileConfig {
    /// Default encoding
    pub encoding: String,

    /// Default line ending
    pub line_ending: String,

    /// Auto-save interval (0 to disable)
    pub auto_save_delay: u32,

    /// Create backup on save
    pub backup_on_save: bool,

    /// Remove trailing whitespace
    pub trim_trailing_whitespace: bool,

    /// Ensure newline at end of file
    pub final_newline: bool,

    /// Watch files for external changes
    pub watch_files: bool,

    /// Max file size to load (MB)
    pub max_file_size: usize,

    /// Patterns to exclude from explorer
    pub exclude_patterns: Vec<String>,
}

impl Default for FileConfig {
    fn default() -> Self {
        Self {
            encoding: "utf-8".to_string(),
            line_ending: "lf".to_string(),
            auto_save_delay: 0,
            backup_on_save: false,
            trim_trailing_whitespace: true,
            final_newline: true,
            watch_files: true,
            max_file_size: 100,
            exclude_patterns: vec![
                "**/.git/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/target/**".to_string(),
                "**/__pycache__/**".to_string(),
            ],
        }
    }
}

/// Keyboard configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardConfig {
    /// Keyboard layout
    pub layout: String,

    /// Key repeat delay (ms)
    pub repeat_delay: u32,

    /// Key repeat rate (chars/sec)
    pub repeat_rate: u32,

    /// Custom key bindings
    #[serde(default)]
    pub bindings: HashMap<String, String>,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            layout: "qwerty".to_string(),
            repeat_delay: 500,
            repeat_rate: 30,
            bindings: HashMap::new(),
        }
    }
}

/// Language-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageConfig {
    /// Tab size for this language
    pub tab_size: Option<usize>,

    /// Use spaces for this language
    pub use_spaces: Option<bool>,

    /// Formatter command
    pub formatter: Option<String>,

    /// LSP server command
    pub lsp: Option<String>,

    /// Comment string
    pub comment: Option<String>,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            tab_size: None,
            use_spaces: None,
            formatter: None,
            lsp: None,
            comment: None,
        }
    }
}

/// Configuration errors.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Config directory not found")]
    NoConfigDir,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.editor.tab_size, 4);
        assert!(config.editor.use_spaces);
        assert_eq!(config.ui.font_size, 14.0);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string(&config).unwrap();
        let parsed: Config = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.editor.tab_size, config.editor.tab_size);
    }
}
