//! Theme system for the editor.
//!
//! ## Learning: Builder Pattern
//!
//! Themes use the builder pattern for flexible construction:
//! ```rust,ignore
//! let theme = Theme::dark()
//!     .with_accent(Color::BLUE)
//!     .with_font_size(14.0);
//! ```

use serde::{Deserialize, Serialize};

/// Color representation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Converts to iced Color.
    pub fn to_iced(&self) -> iced::Color {
        iced::Color::from_rgba(self.r, self.g, self.b, self.a)
    }
}

/// Editor theme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Theme name
    pub name: String,

    /// Is this a dark theme?
    pub is_dark: bool,

    /// Background colors
    pub background: BackgroundColors,

    /// Foreground colors
    pub foreground: ForegroundColors,

    /// UI element colors
    pub ui: UiColors,

    /// Syntax highlighting colors
    pub syntax: SyntaxColors,

    /// Font settings
    pub font: FontSettings,
}

/// Background colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundColors {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub selection: Color,
    pub highlight: Color,
    pub line_highlight: Color,
}

/// Foreground (text) colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForegroundColors {
    pub primary: Color,
    pub secondary: Color,
    pub muted: Color,
    pub accent: Color,
}

/// UI element colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiColors {
    pub border: Color,
    pub divider: Color,
    pub button: Color,
    pub button_hover: Color,
    pub input: Color,
    pub scrollbar: Color,
    pub scrollbar_hover: Color,
}

/// Syntax highlighting colors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxColors {
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub function: Color,
    pub type_name: Color,
    pub variable: Color,
    pub constant: Color,
    pub operator: Color,
    pub punctuation: Color,
    pub attribute: Color,
    pub tag: Color,
}

/// Font settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSettings {
    pub family: String,
    pub size: f32,
    pub line_height: f32,
    pub weight: u16,
}

impl Theme {
    /// Creates the default dark theme.
    pub fn dark() -> Self {
        Self {
            name: "Luminex Dark".to_string(),
            is_dark: true,
            background: BackgroundColors {
                primary: Color::rgb(0.10, 0.10, 0.12),
                secondary: Color::rgb(0.12, 0.12, 0.14),
                tertiary: Color::rgb(0.15, 0.15, 0.17),
                selection: Color::rgba(0.3, 0.5, 0.8, 0.3),
                highlight: Color::rgba(1.0, 1.0, 0.0, 0.1),
                line_highlight: Color::rgba(1.0, 1.0, 1.0, 0.05),
            },
            foreground: ForegroundColors {
                primary: Color::rgb(0.9, 0.9, 0.9),
                secondary: Color::rgb(0.7, 0.7, 0.7),
                muted: Color::rgb(0.5, 0.5, 0.5),
                accent: Color::rgb(0.4, 0.6, 1.0),
            },
            ui: UiColors {
                border: Color::rgb(0.25, 0.25, 0.28),
                divider: Color::rgb(0.2, 0.2, 0.22),
                button: Color::rgb(0.2, 0.2, 0.22),
                button_hover: Color::rgb(0.25, 0.25, 0.28),
                input: Color::rgb(0.15, 0.15, 0.17),
                scrollbar: Color::rgba(1.0, 1.0, 1.0, 0.1),
                scrollbar_hover: Color::rgba(1.0, 1.0, 1.0, 0.2),
            },
            syntax: SyntaxColors {
                keyword: Color::rgb(0.8, 0.5, 0.8),   // Purple
                string: Color::rgb(0.6, 0.8, 0.5),   // Green
                number: Color::rgb(0.9, 0.7, 0.5),   // Orange
                comment: Color::rgb(0.5, 0.5, 0.5),  // Gray
                function: Color::rgb(0.5, 0.7, 0.9), // Blue
                type_name: Color::rgb(0.5, 0.8, 0.8), // Cyan
                variable: Color::rgb(0.9, 0.9, 0.9), // White
                constant: Color::rgb(0.9, 0.6, 0.5), // Red-orange
                operator: Color::rgb(0.9, 0.9, 0.9), // White
                punctuation: Color::rgb(0.7, 0.7, 0.7), // Light gray
                attribute: Color::rgb(0.9, 0.8, 0.5), // Yellow
                tag: Color::rgb(0.8, 0.5, 0.5),      // Red
            },
            font: FontSettings {
                family: "JetBrains Mono".to_string(),
                size: 14.0,
                line_height: 1.5,
                weight: 400,
            },
        }
    }

    /// Creates a light theme.
    pub fn light() -> Self {
        Self {
            name: "Luminex Light".to_string(),
            is_dark: false,
            background: BackgroundColors {
                primary: Color::rgb(1.0, 1.0, 1.0),
                secondary: Color::rgb(0.97, 0.97, 0.97),
                tertiary: Color::rgb(0.95, 0.95, 0.95),
                selection: Color::rgba(0.3, 0.5, 0.8, 0.2),
                highlight: Color::rgba(1.0, 1.0, 0.0, 0.2),
                line_highlight: Color::rgba(0.0, 0.0, 0.0, 0.03),
            },
            foreground: ForegroundColors {
                primary: Color::rgb(0.1, 0.1, 0.1),
                secondary: Color::rgb(0.3, 0.3, 0.3),
                muted: Color::rgb(0.5, 0.5, 0.5),
                accent: Color::rgb(0.2, 0.4, 0.8),
            },
            ui: UiColors {
                border: Color::rgb(0.85, 0.85, 0.85),
                divider: Color::rgb(0.9, 0.9, 0.9),
                button: Color::rgb(0.92, 0.92, 0.92),
                button_hover: Color::rgb(0.88, 0.88, 0.88),
                input: Color::rgb(1.0, 1.0, 1.0),
                scrollbar: Color::rgba(0.0, 0.0, 0.0, 0.1),
                scrollbar_hover: Color::rgba(0.0, 0.0, 0.0, 0.2),
            },
            syntax: SyntaxColors {
                keyword: Color::rgb(0.6, 0.3, 0.6),
                string: Color::rgb(0.3, 0.6, 0.3),
                number: Color::rgb(0.7, 0.4, 0.2),
                comment: Color::rgb(0.5, 0.5, 0.5),
                function: Color::rgb(0.2, 0.4, 0.7),
                type_name: Color::rgb(0.2, 0.6, 0.6),
                variable: Color::rgb(0.1, 0.1, 0.1),
                constant: Color::rgb(0.7, 0.3, 0.2),
                operator: Color::rgb(0.1, 0.1, 0.1),
                punctuation: Color::rgb(0.4, 0.4, 0.4),
                attribute: Color::rgb(0.6, 0.5, 0.2),
                tag: Color::rgb(0.6, 0.2, 0.2),
            },
            font: FontSettings {
                family: "JetBrains Mono".to_string(),
                size: 14.0,
                line_height: 1.5,
                weight: 400,
            },
        }
    }

    /// Loads a theme from a file.
    pub fn load(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Saves the theme to a file.
    pub fn save(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
