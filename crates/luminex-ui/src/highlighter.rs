//! Syntax highlighting integration for the editor.
//!
//! This module provides syntax highlighting using simple pattern matching.

use iced::advanced::text::highlighter::{Format, Highlighter};
use iced::{Color, Font};
use std::ops::Range;

/// Colors for syntax highlighting (dark theme)
mod colors {
    use iced::Color;

    pub const KEYWORD: Color = Color::from_rgb(0.86, 0.55, 0.76);     // Pink/purple
    pub const STRING: Color = Color::from_rgb(0.72, 0.84, 0.55);      // Green
    pub const NUMBER: Color = Color::from_rgb(0.82, 0.68, 0.55);      // Orange
    pub const COMMENT: Color = Color::from_rgb(0.50, 0.55, 0.55);     // Gray
    pub const FUNCTION: Color = Color::from_rgb(0.55, 0.75, 0.90);    // Blue
    pub const TYPE: Color = Color::from_rgb(0.90, 0.80, 0.55);        // Yellow
    pub const VARIABLE: Color = Color::from_rgb(0.85, 0.85, 0.85);    // Light gray
    pub const DEFAULT: Color = Color::from_rgb(0.90, 0.90, 0.90);     // White-ish
}

/// Settings for the highlighter.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HighlightSettings {
    pub language: String,
}

/// Format for highlighted text.
#[derive(Debug, Clone, Copy)]
pub struct HighlightFormat {
    pub color: Color,
}

impl HighlightFormat {
    pub fn to_format(self, _font: Font) -> Format<Font> {
        Format {
            color: Some(self.color),
            font: None,
        }
    }
}

/// Syntax highlighter for the text editor.
pub struct EditorHighlighter {
    current_line: String,
    position: usize,
}

impl Highlighter for EditorHighlighter {
    type Settings = HighlightSettings;
    type Highlight = HighlightFormat;
    type Iterator<'a> = std::iter::Once<(Range<usize>, HighlightFormat)> where Self: 'a;

    fn new(_settings: &Self::Settings) -> Self {
        Self {
            current_line: String::new(),
            position: 0,
        }
    }

    fn update(&mut self, _new_settings: &Self::Settings) {
        // Settings updated
    }

    fn change_line(&mut self, _line: usize) {
        self.position = 0;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        // Store line and reset position
        self.current_line = line.to_string();
        self.position = 0;

        // Find the next highlight span
        if let Some((range, format)) = self.next_span() {
            std::iter::once((range, format))
        } else {
            // Return empty span for empty line
            std::iter::once((0..line.len().max(1), HighlightFormat { color: colors::DEFAULT }))
        }
    }

    fn current_line(&self) -> usize {
        0
    }
}

impl EditorHighlighter {
    fn next_span(&mut self) -> Option<(Range<usize>, HighlightFormat)> {
        let line = &self.current_line;
        if self.position >= line.len() {
            return None;
        }

        let remaining = &line[self.position..];

        // Check for comments
        if remaining.starts_with("//") {
            let start = self.position;
            let len = remaining.len();
            self.position = line.len();
            return Some((start..start + len, HighlightFormat { color: colors::COMMENT }));
        }

        // For simplicity in this first version, just return the whole remaining line
        // with default highlighting and process keywords inline
        let start = self.position;
        let len = remaining.len();
        self.position = line.len();

        // Check if line starts with keyword-like patterns
        let color = if remaining.trim_start().starts_with("fn ") ||
                       remaining.trim_start().starts_with("let ") ||
                       remaining.trim_start().starts_with("pub ") ||
                       remaining.trim_start().starts_with("use ") ||
                       remaining.trim_start().starts_with("mod ") ||
                       remaining.trim_start().starts_with("impl ") ||
                       remaining.trim_start().starts_with("struct ") ||
                       remaining.trim_start().starts_with("enum ") ||
                       remaining.trim_start().starts_with("if ") ||
                       remaining.trim_start().starts_with("else ") ||
                       remaining.trim_start().starts_with("for ") ||
                       remaining.trim_start().starts_with("while ") ||
                       remaining.trim_start().starts_with("match ") ||
                       remaining.trim_start().starts_with("return ") {
            colors::KEYWORD
        } else if remaining.trim_start().starts_with("//") {
            colors::COMMENT
        } else if remaining.contains('"') {
            colors::STRING
        } else {
            colors::DEFAULT
        };

        Some((start..start + len, HighlightFormat { color }))
    }
}

/// Detects language from file extension.
pub fn detect_language(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "py" => "python",
        "js" | "jsx" => "javascript",
        "ts" | "tsx" => "typescript",
        "json" => "json",
        "toml" => "toml",
        "md" => "markdown",
        "html" | "htm" => "html",
        "css" => "css",
        "sh" | "bash" => "bash",
        _ => "text",
    }
}
