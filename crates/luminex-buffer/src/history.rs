//! Undo/redo history management.
//!
//! ## Learning: The Command Pattern
//!
//! Each edit is stored as a command that can be:
//! - Executed (applied to the buffer)
//! - Undone (reversed)
//! - Redone (re-applied after undo)
//!
//! This pattern allows:
//! - Arbitrary undo depth
//! - Edit grouping (multiple edits as one undo step)
//! - Edit coalescing (combining rapid keystrokes)

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// The type of edit operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditKind {
    /// Text was inserted
    Insert,
    /// Text was deleted
    Delete,
}

/// A single edit operation.
///
/// ## Learning: Clone vs Copy
///
/// `Edit` implements `Clone` but not `Copy` because it contains
/// a `String`, which owns heap memory. `Copy` is only for types
/// that can be duplicated with a simple memory copy (like integers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    /// What kind of edit this is
    pub kind: EditKind,
    /// Character position where the edit occurred
    pub position: usize,
    /// The text that was inserted or deleted
    pub content: String,
}

impl Edit {
    /// Creates an insert edit.
    pub fn insert(position: usize, content: impl Into<String>) -> Self {
        Self {
            kind: EditKind::Insert,
            position,
            content: content.into(),
        }
    }

    /// Creates a delete edit.
    pub fn delete(position: usize, content: impl Into<String>) -> Self {
        Self {
            kind: EditKind::Delete,
            position,
            content: content.into(),
        }
    }

    /// Returns the inverse of this edit (for undo).
    pub fn inverse(&self) -> Self {
        Self {
            kind: match self.kind {
                EditKind::Insert => EditKind::Delete,
                EditKind::Delete => EditKind::Insert,
            },
            position: self.position,
            content: self.content.clone(),
        }
    }

    /// Returns true if this edit can be coalesced with another.
    ///
    /// Two edits can be coalesced if:
    /// - They're the same kind
    /// - They're adjacent (next character for insert, same position for delete)
    /// - Neither is a newline
    pub fn can_coalesce(&self, other: &Edit) -> bool {
        if self.kind != other.kind {
            return false;
        }

        // Don't coalesce across newlines
        if self.content.contains('\n') || other.content.contains('\n') {
            return false;
        }

        match self.kind {
            EditKind::Insert => {
                // Can coalesce if other is right after this insert
                self.position + self.content.chars().count() == other.position
            }
            EditKind::Delete => {
                // For backspace: other position + its length == this position
                // For forward delete: same position
                other.position + other.content.chars().count() == self.position
                    || self.position == other.position
            }
        }
    }

    /// Coalesces another edit into this one.
    pub fn coalesce(&mut self, other: Edit) {
        match self.kind {
            EditKind::Insert => {
                self.content.push_str(&other.content);
            }
            EditKind::Delete => {
                if other.position < self.position {
                    // Backspace: prepend
                    self.content = other.content + &self.content;
                    self.position = other.position;
                } else {
                    // Forward delete: append
                    self.content.push_str(&other.content);
                }
            }
        }
    }
}

/// A group of edits that should be undone/redone together.
#[derive(Debug, Clone)]
pub struct EditGroup {
    /// The edits in this group
    pub edits: Vec<Edit>,
    /// When this group was created
    pub timestamp: Option<Instant>,
}

impl EditGroup {
    /// Creates a new edit group.
    pub fn new(edit: Edit) -> Self {
        Self {
            edits: vec![edit],
            timestamp: Some(Instant::now()),
        }
    }

    /// Adds an edit to this group.
    pub fn push(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    /// Returns the last edit in the group.
    #[allow(dead_code)]
    pub fn last(&self) -> Option<&Edit> {
        self.edits.last()
    }

    /// Returns a mutable reference to the last edit.
    pub fn last_mut(&mut self) -> Option<&mut Edit> {
        self.edits.last_mut()
    }
}

/// Manages undo/redo history.
///
/// ## Design Decisions
///
/// 1. **Bounded history**: Limits memory usage for long editing sessions
/// 2. **Edit coalescing**: Combines rapid keystrokes into single undo steps
/// 3. **Edit grouping**: Allows treating multiple operations as one
///
/// ## Learning: VecDeque
///
/// We use `VecDeque` instead of `Vec` because we need efficient:
/// - Push to back (new edits)
/// - Pop from front (when at capacity)
/// - Pop from back (for undo)
#[derive(Debug, Clone)]
pub struct History {
    /// Stack of undoable edit groups
    undo_stack: VecDeque<EditGroup>,
    /// Stack of redoable edit groups
    redo_stack: Vec<EditGroup>,
    /// Maximum number of edit groups to keep
    max_size: usize,
    /// Time threshold for coalescing edits (ms)
    coalesce_threshold: Duration,
    /// Whether we're in a group (for compound operations)
    in_group: bool,
}

impl History {
    /// Creates a new history with the given capacity.
    pub fn new(max_size: usize) -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(max_size),
            redo_stack: Vec::new(),
            max_size,
            coalesce_threshold: Duration::from_millis(300),
            in_group: false,
        }
    }

    /// Pushes an edit onto the history.
    ///
    /// Clears the redo stack (can't redo after new edit).
    /// May coalesce with the previous edit.
    pub fn push(&mut self, edit: Edit) {
        // Clear redo stack - branching history not supported
        self.redo_stack.clear();

        // Try to coalesce with the last edit
        if let Some(last_group) = self.undo_stack.back_mut() {
            if let Some(timestamp) = last_group.timestamp {
                let elapsed = timestamp.elapsed();

                if elapsed < self.coalesce_threshold {
                    if let Some(last_edit) = last_group.last_mut() {
                        if last_edit.can_coalesce(&edit) {
                            last_edit.coalesce(edit);
                            last_group.timestamp = Some(Instant::now());
                            return;
                        }
                    }
                }
            }

            // If in a group, add to current group
            if self.in_group {
                last_group.push(edit);
                last_group.timestamp = Some(Instant::now());
                return;
            }
        }

        // Create new group
        let group = EditGroup::new(edit);
        self.undo_stack.push_back(group);

        // Enforce capacity
        while self.undo_stack.len() > self.max_size {
            self.undo_stack.pop_front();
        }
    }

    /// Starts an edit group.
    ///
    /// All edits until `end_group()` will be treated as one undo step.
    pub fn begin_group(&mut self) {
        self.in_group = true;
    }

    /// Ends the current edit group.
    pub fn end_group(&mut self) {
        self.in_group = false;
    }

    /// Undoes the last edit group.
    ///
    /// Returns the edits to reverse (in reverse order).
    pub fn undo(&mut self) -> Option<Edit> {
        // For simplicity, we return one edit at a time from the group
        // A more sophisticated implementation would return the whole group
        if let Some(group) = self.undo_stack.back_mut() {
            if let Some(edit) = group.edits.pop() {
                // Move to redo stack
                let redo_group = EditGroup {
                    edits: vec![edit.clone()],
                    timestamp: None,
                };

                // If group is now empty, remove it
                if group.edits.is_empty() {
                    self.undo_stack.pop_back();
                }

                // Check if we should merge with existing redo group
                if let Some(last_redo) = self.redo_stack.last_mut() {
                    last_redo.edits.push(edit.clone());
                } else {
                    self.redo_stack.push(redo_group);
                }

                return Some(edit);
            }
        }
        None
    }

    /// Redoes the last undone edit.
    pub fn redo(&mut self) -> Option<Edit> {
        if let Some(group) = self.redo_stack.last_mut() {
            if let Some(edit) = group.edits.pop() {
                // If group is now empty, remove it
                if group.edits.is_empty() {
                    self.redo_stack.pop();
                }

                // Push back to undo stack (without coalescing)
                self.undo_stack.push_back(EditGroup {
                    edits: vec![edit.clone()],
                    timestamp: None, // No timestamp prevents coalescing
                });

                return Some(edit);
            }
        }
        None
    }

    /// Returns true if there are edits to undo.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns true if there are edits to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clears all history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Returns the number of undo steps available.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the number of redo steps available.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_inverse() {
        let insert = Edit::insert(0, "hello");
        let inverse = insert.inverse();

        assert_eq!(inverse.kind, EditKind::Delete);
        assert_eq!(inverse.position, 0);
        assert_eq!(inverse.content, "hello");
    }

    #[test]
    fn test_history_undo_redo() {
        let mut history = History::new(100);

        history.push(Edit::insert(0, "a"));
        std::thread::sleep(Duration::from_millis(400)); // Prevent coalescing
        history.push(Edit::insert(1, "b"));

        assert!(history.can_undo());
        let edit = history.undo().unwrap();
        assert_eq!(edit.content, "b");

        assert!(history.can_redo());
        let edit = history.redo().unwrap();
        assert_eq!(edit.content, "b");
    }

    #[test]
    fn test_edit_coalescing() {
        let mut e1 = Edit::insert(0, "a");
        let e2 = Edit::insert(1, "b");

        assert!(e1.can_coalesce(&e2));
        e1.coalesce(e2);
        assert_eq!(e1.content, "ab");
    }
}
