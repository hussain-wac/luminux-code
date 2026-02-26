//! Event system for editor notifications.
//!
//! ## Learning: Observer Pattern in Rust
//!
//! Rust's ownership model makes traditional observer patterns tricky.
//! We use `tokio::sync::broadcast` for a safe, async-friendly event bus.
//!
//! Key differences from OOP observers:
//! - No object references to manage
//! - Events are values, not callbacks
//! - Subscribers receive copies (Clone)
//! - No lifetime complexity

use crate::document::DocumentId;
use crate::editor::EditorMode;
use tokio::sync::broadcast;

/// Events that can occur in the editor.
///
/// ## Learning: Enum Variants
///
/// Rust enums can hold data, unlike C-style enums.
/// Each variant can have different associated data.
/// Pattern matching ensures all cases are handled.
#[derive(Debug, Clone)]
pub enum EditorEvent {
    // Document events
    /// A document was opened
    DocumentOpened(DocumentId),
    /// A document was closed
    DocumentClosed(DocumentId),
    /// A document was saved
    DocumentSaved(DocumentId),
    /// A document's content changed
    DocumentChanged(DocumentId),
    /// A document received focus
    DocumentFocused(DocumentId),

    // Cursor events
    /// Cursor position changed
    CursorMoved(DocumentId),
    /// Selection changed
    SelectionChanged(DocumentId),

    // Editor events
    /// Editor mode changed
    ModeChanged(EditorMode),
    /// Configuration changed
    ConfigChanged,
    /// Workspace opened
    WorkspaceOpened,
    /// Editor is quitting
    Quit,

    // UI events
    /// Theme changed
    ThemeChanged(String),
    /// Font size changed
    FontSizeChanged(f32),

    // File system events
    /// File changed on disk
    FileChangedOnDisk(std::path::PathBuf),
    /// File deleted on disk
    FileDeletedOnDisk(std::path::PathBuf),
}

/// Event bus for broadcasting editor events.
///
/// ## Design
///
/// Using a broadcast channel allows:
/// - Multiple subscribers (UI, plugins, LSP)
/// - Async reception
/// - No direct coupling between components
/// - Lagged receivers don't block senders
pub struct EventBus {
    sender: broadcast::Sender<EditorEvent>,
}

impl EventBus {
    /// Creates a new event bus.
    pub fn new() -> Self {
        // Capacity of 256 events in the buffer
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }

    /// Emits an event to all subscribers.
    pub fn emit(&self, event: EditorEvent) {
        // Ignore error if no receivers (not a problem)
        let _ = self.sender.send(event);
    }

    /// Subscribes to events.
    ///
    /// Returns a receiver that will get all future events.
    pub fn subscribe(&self) -> broadcast::Receiver<EditorEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

/// Helper for processing events asynchronously.
///
/// ## Example
///
/// ```ignore
/// let mut handler = EventHandler::new(editor.subscribe());
///
/// tokio::spawn(async move {
///     while let Some(event) = handler.next().await {
///         match event {
///             EditorEvent::DocumentChanged(id) => {
///                 // Handle document change
///             }
///             _ => {}
///         }
///     }
/// });
/// ```
pub struct EventHandler {
    receiver: broadcast::Receiver<EditorEvent>,
}

impl EventHandler {
    /// Creates a new event handler.
    pub fn new(receiver: broadcast::Receiver<EditorEvent>) -> Self {
        Self { receiver }
    }

    /// Waits for the next event.
    pub async fn next(&mut self) -> Option<EditorEvent> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Event handler lagged, missed {} events", n);
                    // Continue loop to try again
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.emit(EditorEvent::ConfigChanged);

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, EditorEvent::ConfigChanged));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.emit(EditorEvent::ConfigChanged);

        assert!(rx1.recv().await.is_ok());
        assert!(rx2.recv().await.is_ok());
    }
}
