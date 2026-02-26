//! Text selection handling.
//!
//! ## Learning: Range Types
//!
//! Rust's standard library has `Range<T>` (exclusive end) and
//! `RangeInclusive<T>` (inclusive end). For text, we use exclusive
//! ranges because:
//! - Empty selections (start == end) are natural
//! - Easier arithmetic (length = end - start)
//! - Consistent with slice semantics

use crate::Position;
use serde::{Deserialize, Serialize};

/// A selection of text in the buffer.
///
/// A selection has a start and end position. The start is always
/// before or equal to the end (normalized).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Selection {
    /// Start position (inclusive)
    pub start: Position,
    /// End position (exclusive)
    pub end: Position,
}

impl Selection {
    /// Creates a new selection.
    ///
    /// Automatically normalizes so start <= end.
    pub fn new(start: Position, end: Position) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// Creates a zero-width selection (cursor position).
    pub fn cursor(position: Position) -> Self {
        Self {
            start: position,
            end: position,
        }
    }

    /// Creates a selection spanning entire lines.
    pub fn lines(start_line: usize, end_line: usize) -> Self {
        Self {
            start: Position::new(start_line, 0),
            end: Position::new(end_line + 1, 0),
        }
    }

    /// Returns true if this is a zero-width selection (just a cursor).
    pub fn is_cursor(&self) -> bool {
        self.start == self.end
    }

    /// Returns true if the selection spans multiple lines.
    pub fn is_multiline(&self) -> bool {
        self.start.line != self.end.line
    }

    /// Returns true if a position is within this selection.
    pub fn contains(&self, pos: Position) -> bool {
        pos >= self.start && pos < self.end
    }

    /// Returns true if this selection overlaps with another.
    pub fn overlaps(&self, other: &Selection) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Returns true if this selection is adjacent to another.
    pub fn is_adjacent(&self, other: &Selection) -> bool {
        self.end == other.start || self.start == other.end
    }

    /// Merges this selection with another (if they overlap or are adjacent).
    pub fn merge(&self, other: &Selection) -> Option<Selection> {
        if self.overlaps(other) || self.is_adjacent(other) {
            Some(Selection {
                start: self.start.min(other.start),
                end: self.end.max(other.end),
            })
        } else {
            None
        }
    }

    /// Returns the intersection of two selections.
    pub fn intersect(&self, other: &Selection) -> Option<Selection> {
        if !self.overlaps(other) {
            return None;
        }

        Some(Selection {
            start: self.start.max(other.start),
            end: self.end.min(other.end),
        })
    }

    /// Expands the selection to include full lines.
    pub fn expand_to_lines(&self) -> Selection {
        Selection {
            start: Position::new(self.start.line, 0),
            end: if self.end.column == 0 && self.end.line > self.start.line {
                self.end
            } else {
                Position::new(self.end.line + 1, 0)
            },
        }
    }

    /// Returns the number of lines this selection spans.
    pub fn line_count(&self) -> usize {
        if self.is_cursor() {
            0
        } else {
            self.end.line - self.start.line + if self.end.column > 0 { 1 } else { 0 }
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::cursor(Position::ZERO)
    }
}

/// Represents the direction of a selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionDirection {
    /// Selection extends forward (cursor at end)
    Forward,
    /// Selection extends backward (cursor at start)
    Backward,
}

/// A selection with direction information.
///
/// This preserves whether the user selected forward or backward,
/// which affects where the cursor appears and how selection extends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectedSelection {
    /// The anchor point (where selection started)
    pub anchor: Position,
    /// The active point (where cursor is, selection ends)
    pub active: Position,
}

impl DirectedSelection {
    /// Creates a new directed selection.
    pub fn new(anchor: Position, active: Position) -> Self {
        Self { anchor, active }
    }

    /// Creates a cursor (zero-width selection).
    pub fn cursor(position: Position) -> Self {
        Self {
            anchor: position,
            active: position,
        }
    }

    /// Returns the direction of the selection.
    pub fn direction(&self) -> SelectionDirection {
        if self.anchor <= self.active {
            SelectionDirection::Forward
        } else {
            SelectionDirection::Backward
        }
    }

    /// Returns a normalized (start <= end) selection.
    pub fn to_selection(&self) -> Selection {
        Selection::new(self.anchor, self.active)
    }

    /// Returns the start position.
    pub fn start(&self) -> Position {
        self.anchor.min(self.active)
    }

    /// Returns the end position.
    pub fn end(&self) -> Position {
        self.anchor.max(self.active)
    }

    /// Returns true if this is a zero-width selection.
    pub fn is_cursor(&self) -> bool {
        self.anchor == self.active
    }

    /// Extends the selection to a new active position.
    pub fn extend_to(&mut self, position: Position) {
        self.active = position;
    }
}

/// A collection of selections for multi-cursor editing.
#[derive(Debug, Clone, Default)]
pub struct SelectionSet {
    selections: Vec<DirectedSelection>,
    primary_index: usize,
}

impl SelectionSet {
    /// Creates a new selection set with a single cursor at the origin.
    pub fn new() -> Self {
        Self {
            selections: vec![DirectedSelection::cursor(Position::ZERO)],
            primary_index: 0,
        }
    }

    /// Creates a selection set from a single selection.
    pub fn single(selection: DirectedSelection) -> Self {
        Self {
            selections: vec![selection],
            primary_index: 0,
        }
    }

    /// Returns the primary (most recent) selection.
    pub fn primary(&self) -> &DirectedSelection {
        &self.selections[self.primary_index]
    }

    /// Returns a mutable reference to the primary selection.
    pub fn primary_mut(&mut self) -> &mut DirectedSelection {
        &mut self.selections[self.primary_index]
    }

    /// Returns all selections.
    pub fn all(&self) -> &[DirectedSelection] {
        &self.selections
    }

    /// Returns the number of selections.
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    /// Returns true if there's only one selection.
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    /// Adds a new selection.
    pub fn add(&mut self, selection: DirectedSelection) {
        self.selections.push(selection);
        self.primary_index = self.selections.len() - 1;
        self.normalize();
    }

    /// Removes all selections except the primary one.
    pub fn collapse_to_primary(&mut self) {
        let primary = self.selections[self.primary_index].clone();
        self.selections.clear();
        self.selections.push(primary);
        self.primary_index = 0;
    }

    /// Normalizes selections: sorts and merges overlapping ones.
    fn normalize(&mut self) {
        if self.selections.is_empty() {
            return;
        }

        // Sort by start position
        self.selections
            .sort_by(|a, b| a.start().cmp(&b.start()));

        // Merge overlapping selections
        let mut merged: Vec<DirectedSelection> = vec![self.selections[0]];

        for sel in &self.selections[1..] {
            let last = merged.last_mut().unwrap();
            let last_sel = last.to_selection();
            let current_sel = sel.to_selection();

            if let Some(merged_sel) = last_sel.merge(&current_sel) {
                // Merge: keep the anchor of the first, active of the merged end
                last.active = if last.direction() == SelectionDirection::Forward {
                    merged_sel.end
                } else {
                    merged_sel.start
                };
            } else {
                merged.push(*sel);
            }
        }

        self.selections = merged;
        self.primary_index = self.primary_index.min(self.selections.len() - 1);
    }

    /// Returns selections in reverse order (for applying edits).
    pub fn reverse_order(&self) -> impl Iterator<Item = &DirectedSelection> {
        self.selections.iter().rev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_normalization() {
        let sel = Selection::new(Position::new(2, 5), Position::new(1, 3));
        assert_eq!(sel.start, Position::new(1, 3));
        assert_eq!(sel.end, Position::new(2, 5));
    }

    #[test]
    fn test_selection_contains() {
        let sel = Selection::new(Position::new(1, 0), Position::new(1, 10));
        assert!(sel.contains(Position::new(1, 5)));
        assert!(!sel.contains(Position::new(1, 10))); // End is exclusive
        assert!(!sel.contains(Position::new(2, 0)));
    }

    #[test]
    fn test_selection_merge() {
        let s1 = Selection::new(Position::new(0, 0), Position::new(0, 10));
        let s2 = Selection::new(Position::new(0, 5), Position::new(0, 15));

        let merged = s1.merge(&s2).unwrap();
        assert_eq!(merged.start, Position::new(0, 0));
        assert_eq!(merged.end, Position::new(0, 15));
    }

    #[test]
    fn test_directed_selection() {
        let forward = DirectedSelection::new(Position::new(0, 0), Position::new(0, 10));
        assert_eq!(forward.direction(), SelectionDirection::Forward);

        let backward = DirectedSelection::new(Position::new(0, 10), Position::new(0, 0));
        assert_eq!(backward.direction(), SelectionDirection::Backward);
    }
}
