//! Core text buffer implementation using rope data structure.
//!
//! ## Why Rope?
//!
//! Traditional text editors use gap buffers or arrays, but ropes excel at:
//! - **Large files**: O(log n) insertions/deletions vs O(n) for arrays
//! - **Undo/Redo**: Efficient snapshots without full copies
//! - **Concurrent access**: Immutable chunks can be shared
//!
//! ## Learning: Ownership in Action
//!
//! ```rust,ignore
//! let buffer = TextBuffer::new();  // buffer OWNS the rope
//! let text = buffer.text();        // text BORROWS from buffer
//! // buffer.insert(0, "x");        // ERROR! Can't mutate while borrowed
//! drop(text);                      // Release borrow
//! buffer.insert(0, "x");           // Now OK!
//! ```

use ropey::Rope;
use std::ops::Range;
use std::path::Path;

use crate::history::{Edit, EditKind, History};
use crate::{BufferError, BufferResult, Position};

/// A high-performance text buffer backed by a rope data structure.
///
/// # Thread Safety
///
/// `TextBuffer` is `Send` but not `Sync` - it can be moved between threads
/// but shouldn't be accessed from multiple threads simultaneously.
/// For concurrent editing, clone the buffer or use message passing.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    /// The rope holding our text content
    rope: Rope,

    /// Edit history for undo/redo
    history: History,

    /// Whether the buffer has unsaved changes
    modified: bool,

    /// Associated file path (if any)
    file_path: Option<std::path::PathBuf>,

    /// Buffer-specific settings
    config: BufferConfig,
}

/// Configuration for buffer behavior
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Maximum history entries to keep
    pub max_history: usize,

    /// Tab width in spaces
    pub tab_width: usize,

    /// Use spaces instead of tabs
    pub use_spaces: bool,

    /// Auto-detect indentation from file
    pub detect_indentation: bool,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            max_history: 1000,
            tab_width: 4,
            use_spaces: true,
            detect_indentation: true,
        }
    }
}

impl TextBuffer {
    /// Creates a new empty buffer.
    ///
    /// # Example
    /// ```
    /// use luminex_buffer::TextBuffer;
    ///
    /// let buffer = TextBuffer::new();
    /// assert!(buffer.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            history: History::new(1000),
            modified: false,
            file_path: None,
            config: BufferConfig::default(),
        }
    }

    /// Creates a buffer with custom configuration.
    pub fn with_config(config: BufferConfig) -> Self {
        Self {
            rope: Rope::new(),
            history: History::new(config.max_history),
            modified: false,
            file_path: None,
            config,
        }
    }

    /// Loads a buffer from a file.
    ///
    /// # Learning: Error Handling with `?`
    ///
    /// The `?` operator propagates errors up the call stack.
    /// It's syntactic sugar for:
    /// ```rust,ignore
    /// match result {
    ///     Ok(value) => value,
    ///     Err(e) => return Err(e.into()),
    /// }
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> BufferResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)?;
        let rope = Rope::from_str(&content);

        Ok(Self {
            rope,
            history: History::new(1000),
            modified: false,
            file_path: Some(path.to_path_buf()),
            config: BufferConfig::default(),
        })
    }

    /// Saves the buffer to its associated file.
    pub fn save(&mut self) -> BufferResult<()> {
        let path = self.file_path.clone().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "No file path set")
        })?;
        self.save_as(&path)
    }

    /// Saves the buffer to a specific path.
    pub fn save_as(&mut self, path: impl AsRef<Path>) -> BufferResult<()> {
        let path = path.as_ref();

        // Write to a temporary file first, then rename (atomic write)
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, self.text().as_bytes())?;
        std::fs::rename(&temp_path, path)?;

        self.file_path = Some(path.to_path_buf());
        self.modified = false;
        Ok(())
    }

    // ==================== Text Access ====================

    /// Returns the entire text content as a `Cow<str>`.
    ///
    /// # Learning: Cow (Clone-on-Write)
    ///
    /// For small buffers, this returns a borrowed reference (cheap).
    /// For large buffers spanning multiple rope chunks, it allocates.
    #[inline]
    pub fn text(&self) -> std::borrow::Cow<'_, str> {
        self.rope.slice(..).into()
    }

    /// Returns a specific line (0-indexed).
    ///
    /// Line includes the trailing newline if present.
    pub fn line(&self, line_idx: usize) -> BufferResult<std::borrow::Cow<'_, str>> {
        if line_idx >= self.len_lines() {
            return Err(BufferError::PositionOutOfBounds {
                line: line_idx,
                column: 0,
            });
        }
        Ok(self.rope.line(line_idx).into())
    }

    /// Returns a slice of text by character range.
    pub fn slice(&self, range: Range<usize>) -> BufferResult<std::borrow::Cow<'_, str>> {
        if range.end > self.len_chars() {
            return Err(BufferError::InvalidCharIndex(range.end));
        }
        Ok(self.rope.slice(range).into())
    }

    /// Returns the character at a position.
    pub fn char_at(&self, pos: Position) -> BufferResult<char> {
        let idx = self.position_to_char_idx(pos)?;
        Ok(self.rope.char(idx))
    }

    // ==================== Measurements ====================

    /// Returns true if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Returns the number of characters in the buffer.
    ///
    /// # Note
    /// This counts Unicode grapheme clusters, not bytes.
    #[inline]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Returns the number of bytes in the buffer.
    #[inline]
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Returns the number of lines in the buffer.
    ///
    /// An empty buffer has 1 line. A buffer ending with `\n` counts
    /// the empty line after it.
    #[inline]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Returns the length of a specific line in characters.
    pub fn line_len(&self, line_idx: usize) -> BufferResult<usize> {
        if line_idx >= self.len_lines() {
            return Err(BufferError::PositionOutOfBounds {
                line: line_idx,
                column: 0,
            });
        }
        Ok(self.rope.line(line_idx).len_chars())
    }

    // ==================== Mutations ====================

    /// Inserts text at a character index.
    ///
    /// # Learning: `&mut self`
    ///
    /// This method requires exclusive (mutable) access to the buffer.
    /// Rust's borrow checker ensures no other code can read or write
    /// the buffer while this method executes.
    pub fn insert(&mut self, char_idx: usize, text: &str) -> BufferResult<()> {
        if char_idx > self.len_chars() {
            return Err(BufferError::InvalidCharIndex(char_idx));
        }

        // Record edit for undo
        let edit = Edit {
            kind: EditKind::Insert,
            position: char_idx,
            content: text.to_string(),
        };
        self.history.push(edit);

        // Perform the insertion
        self.rope.insert(char_idx, text);
        self.modified = true;

        Ok(())
    }

    /// Inserts text at a line:column position.
    pub fn insert_at(&mut self, pos: Position, text: &str) -> BufferResult<()> {
        let char_idx = self.position_to_char_idx(pos)?;
        self.insert(char_idx, text)
    }

    /// Deletes text in a character range.
    pub fn delete(&mut self, range: Range<usize>) -> BufferResult<String> {
        if range.end > self.len_chars() {
            return Err(BufferError::InvalidCharIndex(range.end));
        }

        // Capture deleted text for undo
        let deleted: String = self.rope.slice(range.clone()).into();
        let edit = Edit {
            kind: EditKind::Delete,
            position: range.start,
            content: deleted.clone(),
        };
        self.history.push(edit);

        // Perform deletion
        self.rope.remove(range);
        self.modified = true;

        Ok(deleted)
    }

    /// Replaces text in a range with new text.
    pub fn replace(&mut self, range: Range<usize>, text: &str) -> BufferResult<String> {
        // This is a composite operation: delete + insert
        // For undo purposes, we treat it as two separate edits
        let deleted = self.delete(range.clone())?;
        self.insert(range.start, text)?;
        Ok(deleted)
    }

    // ==================== Undo/Redo ====================

    /// Undoes the last edit.
    ///
    /// # Learning: State Management
    ///
    /// Each edit is stored in a history stack. Undo pops from the
    /// undo stack and pushes to the redo stack. This is a classic
    /// "command pattern" implementation.
    pub fn undo(&mut self) -> BufferResult<()> {
        let edit = self.history.undo().ok_or(BufferError::NothingToUndo)?;

        // Apply inverse operation WITHOUT recording to history
        match edit.kind {
            EditKind::Insert => {
                let end = edit.position + edit.content.chars().count();
                self.rope.remove(edit.position..end);
            }
            EditKind::Delete => {
                self.rope.insert(edit.position, &edit.content);
            }
        }

        self.modified = true;
        Ok(())
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) -> BufferResult<()> {
        let edit = self.history.redo().ok_or(BufferError::NothingToRedo)?;

        // Re-apply the operation WITHOUT recording to history
        match edit.kind {
            EditKind::Insert => {
                self.rope.insert(edit.position, &edit.content);
            }
            EditKind::Delete => {
                let end = edit.position + edit.content.chars().count();
                self.rope.remove(edit.position..end);
            }
        }

        self.modified = true;
        Ok(())
    }

    /// Returns true if there are edits to undo.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Returns true if there are edits to redo.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    // ==================== Position Conversion ====================

    /// Converts a Position (line, column) to a character index.
    ///
    /// # Learning: Bounds Checking
    ///
    /// We validate input before operations to maintain invariants.
    /// This prevents panics and provides meaningful error messages.
    pub fn position_to_char_idx(&self, pos: Position) -> BufferResult<usize> {
        if pos.line >= self.len_lines() {
            return Err(BufferError::PositionOutOfBounds {
                line: pos.line,
                column: pos.column,
            });
        }

        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.rope.line(pos.line).len_chars();

        // Allow column to be at end of line (for insertion)
        if pos.column > line_len {
            return Err(BufferError::PositionOutOfBounds {
                line: pos.line,
                column: pos.column,
            });
        }

        Ok(line_start + pos.column)
    }

    /// Converts a character index to a Position (line, column).
    pub fn char_idx_to_position(&self, char_idx: usize) -> BufferResult<Position> {
        if char_idx > self.len_chars() {
            return Err(BufferError::InvalidCharIndex(char_idx));
        }

        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        let column = char_idx - line_start;

        Ok(Position { line, column })
    }

    // ==================== State Queries ====================

    /// Returns true if the buffer has unsaved changes.
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    /// Returns the associated file path, if any.
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Returns the buffer's configuration.
    pub fn config(&self) -> &BufferConfig {
        &self.config
    }

    /// Sets a new configuration.
    pub fn set_config(&mut self, config: BufferConfig) {
        self.config = config;
    }

    // ==================== Search ====================

    /// Finds all occurrences of a pattern.
    ///
    /// Returns character indices of each match start.
    pub fn find_all(&self, pattern: &str) -> Vec<usize> {
        let text: String = self.rope.slice(..).into();
        text.match_indices(pattern)
            .map(|(byte_idx, _)| {
                // Convert byte index to char index
                text[..byte_idx].chars().count()
            })
            .collect()
    }

    /// Finds the next occurrence starting from a position.
    pub fn find_next(&self, pattern: &str, from: usize) -> Option<usize> {
        let text: String = self.rope.slice(..).into();
        let byte_from = text.char_indices().nth(from).map(|(i, _)| i).unwrap_or(0);

        text[byte_from..]
            .find(pattern)
            .map(|byte_offset| from + text[byte_from..byte_from + byte_offset].chars().count())
    }

    /// Replaces all occurrences of a pattern.
    pub fn replace_all(&mut self, pattern: &str, replacement: &str) -> usize {
        let matches = self.find_all(pattern);
        let pattern_len = pattern.chars().count();
        let replacement_len = replacement.chars().count();
        let mut offset: isize = 0;

        for match_idx in &matches {
            let adjusted_idx = (*match_idx as isize + offset) as usize;
            let _ = self.replace(adjusted_idx..adjusted_idx + pattern_len, replacement);
            offset += replacement_len as isize - pattern_len as isize;
        }

        matches.len()
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for TextBuffer {
    fn from(s: &str) -> Self {
        Self {
            rope: Rope::from_str(s),
            history: History::new(1000),
            modified: false,
            file_path: None,
            config: BufferConfig::default(),
        }
    }
}

impl From<String> for TextBuffer {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}
