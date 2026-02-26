//! Cursor and position types for text navigation.
//!
//! ## Learning: Newtype Pattern
//!
//! `Position` is a struct that wraps line/column coordinates.
//! This is better than using `(usize, usize)` because:
//! - Type safety: Can't accidentally swap line and column
//! - Named fields: Self-documenting code
//! - Methods: Can add behavior specific to positions

use serde::{Deserialize, Serialize};

/// A position in the text buffer (line and column).
///
/// Both line and column are 0-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Position {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed, in characters not bytes)
    pub column: usize,
}

impl Position {
    /// Creates a new position.
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Position at the start of the document.
    pub const ZERO: Position = Position { line: 0, column: 0 };

    /// Returns true if this position is before another.
    pub fn is_before(&self, other: &Position) -> bool {
        self.line < other.line || (self.line == other.line && self.column < other.column)
    }

    /// Returns true if this position is after another.
    pub fn is_after(&self, other: &Position) -> bool {
        other.is_before(self)
    }

    /// Returns the earlier of two positions.
    pub fn min(self, other: Position) -> Position {
        if self.is_before(&other) {
            self
        } else {
            other
        }
    }

    /// Returns the later of two positions.
    pub fn max(self, other: Position) -> Position {
        if self.is_after(&other) {
            self
        } else {
            other
        }
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Position {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.line.cmp(&other.line) {
            std::cmp::Ordering::Equal => self.column.cmp(&other.column),
            other => other,
        }
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display as 1-indexed for user-facing output
        write!(f, "{}:{}", self.line + 1, self.column + 1)
    }
}

/// A cursor in the text buffer with position and optional selection anchor.
///
/// ## Learning: Affinity
///
/// When a cursor is at a line break, it could visually appear at the end
/// of one line or the start of the next. `affinity` tracks the intended
/// visual position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    /// Current cursor position
    pub position: Position,

    /// Selection anchor (if selecting text)
    /// When Some, text between anchor and position is selected.
    pub anchor: Option<Position>,

    /// Preferred column for vertical movement
    /// When moving up/down, cursor tries to maintain this column.
    pub preferred_column: Option<usize>,

    /// Visual affinity at line boundaries
    pub affinity: Affinity,
}

/// Visual affinity for cursor at line boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Affinity {
    /// Cursor prefers to appear at end of current line
    #[default]
    Backward,
    /// Cursor prefers to appear at start of next line
    Forward,
}

impl Cursor {
    /// Creates a new cursor at a position.
    pub fn new(position: Position) -> Self {
        Self {
            position,
            anchor: None,
            preferred_column: None,
            affinity: Affinity::Backward,
        }
    }

    /// Creates a cursor at line 0, column 0.
    pub fn at_start() -> Self {
        Self::new(Position::ZERO)
    }

    /// Moves the cursor to a new position, clearing selection.
    pub fn move_to(&mut self, position: Position) {
        self.position = position;
        self.anchor = None;
        self.preferred_column = None;
    }

    /// Moves the cursor, extending selection from current position.
    pub fn select_to(&mut self, position: Position) {
        if self.anchor.is_none() {
            self.anchor = Some(self.position);
        }
        self.position = position;
    }

    /// Clears any selection, keeping cursor position.
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Returns true if text is selected.
    pub fn has_selection(&self) -> bool {
        self.anchor.is_some() && self.anchor != Some(self.position)
    }

    /// Returns the selection range (start, end) if text is selected.
    /// Start is always before end, regardless of selection direction.
    pub fn selection_range(&self) -> Option<(Position, Position)> {
        self.anchor.map(|anchor| {
            if anchor.is_before(&self.position) {
                (anchor, self.position)
            } else {
                (self.position, anchor)
            }
        })
    }

    /// Selects all text (sets anchor to start, cursor to given end).
    pub fn select_all(&mut self, end: Position) {
        self.anchor = Some(Position::ZERO);
        self.position = end;
    }

    /// Selects the current line.
    pub fn select_line(&mut self, line: usize, line_len: usize) {
        self.anchor = Some(Position::new(line, 0));
        self.position = Position::new(line, line_len);
    }

    /// Moves cursor up by the given number of lines.
    pub fn move_up(&mut self, lines: usize) {
        if self.position.line >= lines {
            self.position.line -= lines;
        } else {
            self.position.line = 0;
        }
        self.anchor = None;
        // Keep preferred_column for consistent vertical movement
    }

    /// Moves cursor down by the given number of lines.
    pub fn move_down(&mut self, lines: usize, max_line: usize) {
        self.position.line = (self.position.line + lines).min(max_line);
        self.anchor = None;
    }

    /// Moves cursor left by the given number of columns.
    pub fn move_left(&mut self, cols: usize) {
        if self.position.column >= cols {
            self.position.column -= cols;
        } else if self.position.line > 0 {
            // Wrap to end of previous line (actual column set by caller)
            self.position.line -= 1;
            self.position.column = usize::MAX; // Signal to caller to set to line end
        } else {
            self.position.column = 0;
        }
        self.anchor = None;
        self.preferred_column = None;
    }

    /// Moves cursor right by the given number of columns.
    pub fn move_right(&mut self, cols: usize, line_len: usize, is_last_line: bool) {
        let new_col = self.position.column + cols;
        if new_col <= line_len {
            self.position.column = new_col;
        } else if !is_last_line {
            // Wrap to start of next line
            self.position.line += 1;
            self.position.column = 0;
        } else {
            self.position.column = line_len;
        }
        self.anchor = None;
        self.preferred_column = None;
    }
}

/// Collection of multiple cursors for multi-cursor editing.
///
/// ## Learning: Multi-Cursor Design
///
/// Each cursor is independent, but operations must be applied
/// in reverse position order to avoid invalidating indices.
#[derive(Debug, Clone, Default)]
pub struct MultiCursor {
    /// All cursors, kept sorted by position
    cursors: Vec<Cursor>,
    /// Index of the "primary" cursor (most recently added)
    primary: usize,
}

impl MultiCursor {
    /// Creates a multi-cursor with a single cursor at the start.
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::at_start()],
            primary: 0,
        }
    }

    /// Creates from a single cursor.
    pub fn from_cursor(cursor: Cursor) -> Self {
        Self {
            cursors: vec![cursor],
            primary: 0,
        }
    }

    /// Returns the primary cursor.
    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary]
    }

    /// Returns a mutable reference to the primary cursor.
    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary]
    }

    /// Returns all cursors.
    pub fn all(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Returns mutable access to all cursors.
    pub fn all_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Returns the number of cursors.
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    /// Returns true if there's only one cursor.
    pub fn is_single(&self) -> bool {
        self.cursors.len() == 1
    }

    /// Adds a new cursor at a position.
    /// Returns false if a cursor already exists at that position.
    pub fn add(&mut self, position: Position) -> bool {
        // Check for duplicate
        if self
            .cursors
            .iter()
            .any(|c| c.position == position && !c.has_selection())
        {
            return false;
        }

        let cursor = Cursor::new(position);
        self.cursors.push(cursor);
        self.primary = self.cursors.len() - 1;
        self.sort();
        true
    }

    /// Removes all cursors except the primary one.
    pub fn collapse_to_primary(&mut self) {
        let primary = self.cursors[self.primary].clone();
        self.cursors.clear();
        self.cursors.push(primary);
        self.primary = 0;
    }

    /// Sorts cursors by position and removes duplicates.
    fn sort(&mut self) {
        let primary_pos = self.cursors[self.primary].position;

        self.cursors.sort_by(|a, b| a.position.cmp(&b.position));

        // Deduplicate
        self.cursors.dedup_by(|a, b| {
            a.position == b.position && !a.has_selection() && !b.has_selection()
        });

        // Update primary index
        self.primary = self
            .cursors
            .iter()
            .position(|c| c.position == primary_pos)
            .unwrap_or(0);
    }

    /// Returns cursors in reverse order (for applying edits).
    ///
    /// When editing, we must apply changes from end to start
    /// so that earlier changes don't invalidate later positions.
    pub fn reverse_order(&self) -> impl Iterator<Item = &Cursor> {
        self.cursors.iter().rev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_ordering() {
        let p1 = Position::new(1, 5);
        let p2 = Position::new(2, 3);
        let p3 = Position::new(1, 10);

        assert!(p1.is_before(&p2));
        assert!(p1.is_before(&p3));
        assert!(p2.is_after(&p1));
        assert!(p2.is_after(&p3));
    }

    #[test]
    fn test_cursor_selection() {
        let mut cursor = Cursor::new(Position::new(1, 5));
        assert!(!cursor.has_selection());

        cursor.select_to(Position::new(2, 3));
        assert!(cursor.has_selection());

        let (start, end) = cursor.selection_range().unwrap();
        assert_eq!(start, Position::new(1, 5));
        assert_eq!(end, Position::new(2, 3));
    }

    #[test]
    fn test_multi_cursor() {
        let mut mc = MultiCursor::new();
        mc.add(Position::new(1, 0));
        mc.add(Position::new(2, 0));

        assert_eq!(mc.len(), 3); // Initial + 2 added
    }
}
