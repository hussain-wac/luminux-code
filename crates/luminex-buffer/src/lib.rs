//! # Luminex Buffer
//!
//! High-performance text buffer using rope data structure.
//!
//! ## Key Concepts for Learning Rust
//!
//! ### Ownership & Borrowing
//! - `TextBuffer` owns the rope data structure
//! - Methods like `text()` return borrowed references (`&str`)
//! - Mutations require `&mut self` (exclusive access)
//!
//! ### Memory Safety
//! - No manual memory management needed
//! - Rope handles internal memory efficiently
//! - Cursor positions are validated to prevent out-of-bounds access

mod buffer;
mod cursor;
mod history;
mod selection;

pub use buffer::TextBuffer;
pub use cursor::{Cursor, MultiCursor, Position};
pub use history::{Edit, EditKind, History};
pub use selection::Selection;

/// Result type for buffer operations
pub type BufferResult<T> = Result<T, BufferError>;

/// Errors that can occur during buffer operations
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Position {line}:{column} is out of bounds")]
    PositionOutOfBounds { line: usize, column: usize },

    #[error("Invalid byte index: {0}")]
    InvalidByteIndex(usize),

    #[error("Invalid character index: {0}")]
    InvalidCharIndex(usize),

    #[error("Selection is invalid: start {start:?} is after end {end:?}")]
    InvalidSelection { start: Position, end: Position },

    #[error("Nothing to undo")]
    NothingToUndo,

    #[error("Nothing to redo")]
    NothingToRedo,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = TextBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len_chars(), 0);
    }

    #[test]
    fn test_buffer_from_string() {
        let buffer = TextBuffer::from("Hello, World!");
        assert_eq!(buffer.len_chars(), 13);
        assert_eq!(buffer.text(), "Hello, World!");
    }

    #[test]
    fn test_insert_and_delete() {
        let mut buffer = TextBuffer::new();
        buffer.insert(0, "Hello").unwrap();
        assert_eq!(buffer.text(), "Hello");

        buffer.insert(5, ", World!").unwrap();
        assert_eq!(buffer.text(), "Hello, World!");

        buffer.delete(5..7).unwrap();
        assert_eq!(buffer.text(), "HelloWorld!");
    }

    #[test]
    fn test_undo_redo() {
        let mut buffer = TextBuffer::new();
        buffer.insert(0, "Hello").unwrap();
        buffer.insert(5, " World").unwrap();

        assert_eq!(buffer.text(), "Hello World");

        buffer.undo().unwrap();
        assert_eq!(buffer.text(), "Hello");

        buffer.redo().unwrap();
        assert_eq!(buffer.text(), "Hello World");
    }

    #[test]
    fn test_line_operations() {
        let buffer = TextBuffer::from("Line 1\nLine 2\nLine 3");
        assert_eq!(buffer.len_lines(), 3);
        assert_eq!(buffer.line(0).unwrap(), "Line 1\n");
        assert_eq!(buffer.line(1).unwrap(), "Line 2\n");
        assert_eq!(buffer.line(2).unwrap(), "Line 3");
    }
}
