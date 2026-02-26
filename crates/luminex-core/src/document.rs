//! Document management.
//!
//! ## Learning: Type Aliases and Newtypes
//!
//! `DocumentId` is a newtype wrapper around `Uuid`. This provides:
//! - Type safety: Can't accidentally use a string as a document ID
//! - Encapsulation: Can change the underlying type without breaking APIs
//! - Documentation: The type name explains its purpose

use luminex_buffer::{Cursor, MultiCursor, Position, TextBuffer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::{CoreError, CoreResult};

/// Unique identifier for a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(Uuid);

impl DocumentId {
    /// Creates a new unique document ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A document represents a single file or buffer being edited.
///
/// ## Learning: Composition over Inheritance
///
/// Rust doesn't have inheritance. Instead, `Document` composes
/// a `TextBuffer` and adds document-specific functionality.
/// This is cleaner and more flexible than inheritance.
pub struct Document {
    /// Unique identifier
    id: DocumentId,

    /// The underlying text buffer
    buffer: TextBuffer,

    /// Cursor state
    cursors: MultiCursor,

    /// File path (None for untitled documents)
    path: Option<PathBuf>,

    /// Display name
    name: String,

    /// Document language (for syntax highlighting)
    language: Option<String>,

    /// Line ending style
    line_ending: LineEnding,

    /// Encoding
    encoding: String,

    /// Tab settings
    tab_config: TabConfig,
}

/// Line ending style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEnding {
    /// Unix-style: \n
    #[default]
    Lf,
    /// Windows-style: \r\n
    CrLf,
    /// Classic Mac: \r
    Cr,
}

impl LineEnding {
    /// Returns the string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }

    /// Detects line ending from text.
    pub fn detect(text: &str) -> Self {
        if text.contains("\r\n") {
            LineEnding::CrLf
        } else if text.contains('\r') {
            LineEnding::Cr
        } else {
            LineEnding::Lf
        }
    }
}

/// Tab configuration.
#[derive(Debug, Clone, Copy)]
pub struct TabConfig {
    /// Tab width in spaces
    pub width: usize,
    /// Use spaces instead of tabs
    pub use_spaces: bool,
}

impl Default for TabConfig {
    fn default() -> Self {
        Self {
            width: 4,
            use_spaces: true,
        }
    }
}

impl Document {
    /// Creates a new empty document.
    pub fn new() -> Self {
        Self {
            id: DocumentId::new(),
            buffer: TextBuffer::new(),
            cursors: MultiCursor::new(),
            path: None,
            name: "Untitled".to_string(),
            language: None,
            line_ending: LineEnding::default(),
            encoding: "utf-8".to_string(),
            tab_config: TabConfig::default(),
        }
    }

    /// Opens a document from a file.
    pub fn from_file(path: impl AsRef<Path>) -> CoreResult<Self> {
        let path = path.as_ref();
        let buffer = TextBuffer::from_file(path)?;

        // Detect line ending from file content
        let line_ending = LineEnding::detect(&buffer.text());

        // Get file name
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        // Detect language from extension
        let language = path
            .extension()
            .and_then(|e| e.to_str())
            .map(Self::language_from_extension);

        Ok(Self {
            id: DocumentId::new(),
            buffer,
            cursors: MultiCursor::new(),
            path: Some(path.to_path_buf()),
            name,
            language,
            line_ending,
            encoding: "utf-8".to_string(),
            tab_config: TabConfig::default(),
        })
    }

    /// Detects language from file extension.
    fn language_from_extension(ext: &str) -> String {
        match ext.to_lowercase().as_str() {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "jsx" => "javascript",
            "tsx" => "typescript",
            "html" | "htm" => "html",
            "css" => "css",
            "json" => "json",
            "yaml" | "yml" => "yaml",
            "toml" => "toml",
            "md" | "markdown" => "markdown",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" | "cxx" => "cpp",
            "go" => "go",
            "java" => "java",
            "rb" => "ruby",
            "sh" | "bash" | "zsh" => "bash",
            _ => ext,
        }
        .to_string()
    }

    // ==================== Getters ====================

    /// Returns the document ID.
    pub fn id(&self) -> DocumentId {
        self.id
    }

    /// Returns the file path.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the detected language.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Returns true if the document has unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.buffer.is_modified()
    }

    /// Returns the text buffer.
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// Returns a mutable reference to the buffer.
    pub fn buffer_mut(&mut self) -> &mut TextBuffer {
        &mut self.buffer
    }

    /// Returns the cursor state.
    pub fn cursors(&self) -> &MultiCursor {
        &self.cursors
    }

    /// Returns the primary cursor position.
    pub fn cursor_position(&self) -> Position {
        self.cursors.primary().position
    }

    /// Returns the line count.
    pub fn line_count(&self) -> usize {
        self.buffer.len_lines()
    }

    /// Returns a specific line's content.
    pub fn line(&self, line: usize) -> CoreResult<std::borrow::Cow<'_, str>> {
        Ok(self.buffer.line(line)?)
    }

    /// Returns all text.
    pub fn text(&self) -> std::borrow::Cow<'_, str> {
        self.buffer.text()
    }

    // ==================== File Operations ====================

    /// Saves the document.
    pub fn save(&mut self) -> CoreResult<()> {
        self.buffer.save()?;
        Ok(())
    }

    /// Saves the document to a new path.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> CoreResult<()> {
        let path = path.as_ref();
        self.buffer.save_as(path)?;
        self.path = Some(path.to_path_buf());
        self.name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        self.language = path
            .extension()
            .and_then(|e| e.to_str())
            .map(Self::language_from_extension);
        Ok(())
    }

    // ==================== Text Editing ====================

    /// Inserts text at the current cursor position.
    pub fn insert_at_cursor(&mut self, text: &str) -> CoreResult<()> {
        let pos = self.cursor_position();
        let idx = self.buffer.position_to_char_idx(pos)?;
        self.buffer.insert(idx, text)?;

        // Move cursor past inserted text
        let new_idx = idx + text.chars().count();
        let new_pos = self.buffer.char_idx_to_position(new_idx)?;
        self.cursors.primary_mut().move_to(new_pos);

        Ok(())
    }

    /// Deletes the character before the cursor (backspace).
    pub fn delete_backward(&mut self) -> CoreResult<()> {
        let cursor = self.cursors.primary();

        if cursor.has_selection() {
            return self.delete_selection();
        }

        let pos = cursor.position;
        let idx = self.buffer.position_to_char_idx(pos)?;

        if idx > 0 {
            self.buffer.delete(idx - 1..idx)?;
            let new_pos = self.buffer.char_idx_to_position(idx - 1)?;
            self.cursors.primary_mut().move_to(new_pos);
        }

        Ok(())
    }

    /// Deletes the character after the cursor (delete key).
    pub fn delete_forward(&mut self) -> CoreResult<()> {
        let cursor = self.cursors.primary();

        if cursor.has_selection() {
            return self.delete_selection();
        }

        let pos = cursor.position;
        let idx = self.buffer.position_to_char_idx(pos)?;

        if idx < self.buffer.len_chars() {
            self.buffer.delete(idx..idx + 1)?;
        }

        Ok(())
    }

    /// Deletes the current selection.
    pub fn delete_selection(&mut self) -> CoreResult<()> {
        if let Some((start, end)) = self.cursors.primary().selection_range() {
            let start_idx = self.buffer.position_to_char_idx(start)?;
            let end_idx = self.buffer.position_to_char_idx(end)?;
            self.buffer.delete(start_idx..end_idx)?;
            self.cursors.primary_mut().move_to(start);
        }
        Ok(())
    }

    /// Returns the selected text.
    pub fn selected_text(&self) -> Option<String> {
        let cursor = self.cursors.primary();
        cursor.selection_range().and_then(|(start, end)| {
            let start_idx = self.buffer.position_to_char_idx(start).ok()?;
            let end_idx = self.buffer.position_to_char_idx(end).ok()?;
            self.buffer.slice(start_idx..end_idx).ok().map(|s| s.into_owned())
        })
    }

    /// Inserts a new line.
    pub fn insert_newline(&mut self) -> CoreResult<()> {
        self.insert_at_cursor(self.line_ending.as_str())
    }

    // ==================== Undo/Redo ====================

    /// Undoes the last action.
    pub fn undo(&mut self) -> CoreResult<()> {
        self.buffer.undo()?;
        Ok(())
    }

    /// Redoes the last undone action.
    pub fn redo(&mut self) -> CoreResult<()> {
        self.buffer.redo()?;
        Ok(())
    }

    // ==================== Cursor Movement ====================

    /// Moves the cursor up by n lines.
    pub fn move_cursor_up(&mut self, n: usize) {
        self.cursors.primary_mut().move_up(n);
        self.clamp_cursor_to_line();
    }

    /// Moves the cursor down by n lines.
    pub fn move_cursor_down(&mut self, n: usize) {
        let max_line = self.buffer.len_lines().saturating_sub(1);
        self.cursors.primary_mut().move_down(n, max_line);
        self.clamp_cursor_to_line();
    }

    /// Moves the cursor left by n characters.
    pub fn move_cursor_left(&mut self, n: usize) {
        self.cursors.primary_mut().move_left(n);
        self.clamp_cursor_to_line();
    }

    /// Moves the cursor right by n characters.
    pub fn move_cursor_right(&mut self, n: usize) {
        let pos = self.cursor_position();
        let line_len = self.buffer.line_len(pos.line).unwrap_or(0);
        let is_last = pos.line >= self.buffer.len_lines().saturating_sub(1);
        self.cursors.primary_mut().move_right(n, line_len, is_last);
        self.clamp_cursor_to_line();
    }

    /// Moves cursor to the start of the current line.
    pub fn move_to_line_start(&mut self) {
        let pos = self.cursor_position();
        self.cursors.primary_mut().move_to(Position::new(pos.line, 0));
    }

    /// Moves cursor to the end of the current line.
    pub fn move_to_line_end(&mut self) {
        let pos = self.cursor_position();
        if let Ok(line_len) = self.buffer.line_len(pos.line) {
            // Exclude newline character
            let col = if line_len > 0 {
                let line = self.buffer.line(pos.line).unwrap_or_default();
                if line.ends_with('\n') {
                    line_len - 1
                } else {
                    line_len
                }
            } else {
                0
            };
            self.cursors.primary_mut().move_to(Position::new(pos.line, col));
        }
    }

    /// Moves cursor to a specific position.
    pub fn move_cursor_to(&mut self, pos: Position) {
        self.cursors.primary_mut().move_to(pos);
        self.clamp_cursor_to_line();
    }

    /// Clamps cursor column to current line length.
    fn clamp_cursor_to_line(&mut self) {
        let pos = self.cursor_position();
        if let Ok(line_len) = self.buffer.line_len(pos.line) {
            let max_col = line_len.saturating_sub(1);
            if pos.column > max_col && line_len > 0 {
                self.cursors.primary_mut().position.column = max_col;
            }
        }
    }

    // ==================== Selection ====================

    /// Selects all text.
    pub fn select_all(&mut self) {
        let end_line = self.buffer.len_lines().saturating_sub(1);
        let end_col = self.buffer.line_len(end_line).unwrap_or(0);
        let end = Position::new(end_line, end_col);
        self.cursors.primary_mut().select_all(end);
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.cursors.primary_mut().clear_selection();
    }

    /// Extends selection to a position.
    pub fn select_to(&mut self, pos: Position) {
        self.cursors.primary_mut().select_to(pos);
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages multiple open documents.
pub struct DocumentManager {
    /// All open documents
    documents: HashMap<DocumentId, Document>,

    /// Order of documents (for tabs)
    order: Vec<DocumentId>,

    /// Currently active document
    active: Option<DocumentId>,
}

impl DocumentManager {
    /// Creates a new document manager.
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            order: Vec::new(),
            active: None,
        }
    }

    /// Adds a document.
    pub fn add(&mut self, doc: Document) -> DocumentId {
        let id = doc.id();
        self.documents.insert(id, doc);
        self.order.push(id);
        self.active = Some(id);
        id
    }

    /// Removes a document.
    pub fn close(&mut self, id: DocumentId) -> CoreResult<()> {
        if !self.documents.contains_key(&id) {
            return Err(CoreError::DocumentNotFound(id));
        }

        self.documents.remove(&id);
        self.order.retain(|&i| i != id);

        if self.active == Some(id) {
            self.active = self.order.last().copied();
        }

        Ok(())
    }

    /// Returns a document by ID.
    pub fn get(&self, id: DocumentId) -> Option<&Document> {
        self.documents.get(&id)
    }

    /// Returns a mutable document by ID.
    pub fn get_mut(&mut self, id: DocumentId) -> Option<&mut Document> {
        self.documents.get_mut(&id)
    }

    /// Returns the active document.
    pub fn active(&self) -> Option<&Document> {
        self.active.and_then(|id| self.documents.get(&id))
    }

    /// Returns a mutable reference to the active document.
    pub fn active_mut(&mut self) -> Option<&mut Document> {
        self.active.and_then(|id| self.documents.get_mut(&id))
    }

    /// Sets the active document.
    pub fn set_active(&mut self, id: DocumentId) {
        if self.documents.contains_key(&id) {
            self.active = Some(id);
        }
    }

    /// Finds a document by path.
    pub fn find_by_path(&self, path: &Path) -> Option<DocumentId> {
        self.documents
            .iter()
            .find(|(_, doc)| doc.path() == Some(path))
            .map(|(&id, _)| id)
    }

    /// Returns an iterator over all documents.
    pub fn iter(&self) -> impl Iterator<Item = &Document> {
        self.documents.values()
    }

    /// Returns the document order (for tabs).
    pub fn order(&self) -> &[DocumentId] {
        &self.order
    }

    /// Returns the number of open documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Returns true if no documents are open.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}
