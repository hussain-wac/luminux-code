//! Style definitions for UI components.

/// Styles for the editor view.
pub struct EditorStyle {
    pub gutter_width: f32,
    pub line_number_padding: f32,
    pub cursor_width: f32,
    pub cursor_blink_rate: u32,
}

impl Default for EditorStyle {
    fn default() -> Self {
        Self {
            gutter_width: 60.0,
            line_number_padding: 8.0,
            cursor_width: 2.0,
            cursor_blink_rate: 500,
        }
    }
}
