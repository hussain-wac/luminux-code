//! Main editor orchestration.
//!
//! ## Learning: The Facade Pattern
//!
//! `Editor` acts as a facade, providing a simple interface to
//! complex subsystems. External code only needs to interact with
//! `Editor`, not individual components.

use std::path::Path;

use crate::command::CommandRegistry;
use crate::config::Config;
use crate::document::{Document, DocumentId, DocumentManager};
use crate::event::{EditorEvent, EventBus};
use crate::keymap::Keymap;
use crate::workspace::Workspace;
use crate::{CoreError, CoreResult};

/// The main editor state.
///
/// ## Thread Safety
///
/// `Editor` is designed to be owned by a single thread (the main/UI thread).
/// Background operations (file loading, LSP) communicate via channels.
///
/// ## Learning: Interior Mutability
///
/// Some fields use `RefCell` or similar for interior mutability when needed.
/// This allows mutation through shared references, but panics if borrowed
/// incorrectly. Prefer explicit `&mut` when possible.
pub struct Editor {
    /// Document management
    documents: DocumentManager,

    /// Current workspace
    workspace: Option<Workspace>,

    /// Editor configuration
    config: Config,

    /// Key bindings
    keymap: Keymap,

    /// Command registry
    #[allow(dead_code)]
    commands: CommandRegistry,

    /// Event bus for notifications
    event_bus: EventBus,

    /// Editor mode (normal, insert, etc.)
    mode: EditorMode,

    /// Clipboard content
    clipboard: String,

    /// Whether the editor should quit
    should_quit: bool,
}

/// Editor modes (inspired by modal editors like Vim).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Normal mode - navigation and commands
    #[default]
    Normal,
    /// Insert mode - typing text
    Insert,
    /// Visual mode - selecting text
    Visual,
    /// Command mode - entering commands
    Command,
}

impl Editor {
    /// Creates a new editor instance.
    pub fn new() -> Self {
        Self {
            documents: DocumentManager::new(),
            workspace: None,
            config: Config::default(),
            keymap: Keymap::default(),
            commands: CommandRegistry::new(),
            event_bus: EventBus::new(),
            mode: EditorMode::default(),
            clipboard: String::new(),
            should_quit: false,
        }
    }

    /// Creates an editor with custom configuration.
    pub fn with_config(config: Config) -> Self {
        let keymap = Keymap::from_config(&config);
        Self {
            documents: DocumentManager::new(),
            workspace: None,
            config,
            keymap,
            commands: CommandRegistry::new(),
            event_bus: EventBus::new(),
            mode: EditorMode::default(),
            clipboard: String::new(),
            should_quit: false,
        }
    }

    // ==================== Document Operations ====================

    /// Opens a file in a new document.
    pub fn open_file(&mut self, path: impl AsRef<Path>) -> CoreResult<DocumentId> {
        let path = path.as_ref();

        // Check if already open
        if let Some(id) = self.documents.find_by_path(path) {
            self.documents.set_active(id);
            self.emit(EditorEvent::DocumentFocused(id));
            return Ok(id);
        }

        // Create new document
        let doc = Document::from_file(path)?;
        let id = self.documents.add(doc);
        self.documents.set_active(id);

        self.emit(EditorEvent::DocumentOpened(id));
        self.emit(EditorEvent::DocumentFocused(id));

        Ok(id)
    }

    /// Creates a new untitled document.
    pub fn new_document(&mut self) -> DocumentId {
        let doc = Document::new();
        let id = self.documents.add(doc);
        self.documents.set_active(id);

        self.emit(EditorEvent::DocumentOpened(id));
        self.emit(EditorEvent::DocumentFocused(id));

        id
    }

    /// Closes a document.
    pub fn close_document(&mut self, id: DocumentId) -> CoreResult<()> {
        self.documents.close(id)?;
        self.emit(EditorEvent::DocumentClosed(id));
        Ok(())
    }

    /// Saves the current document.
    pub fn save_current(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.save()?;
        let id = doc.id();
        self.emit(EditorEvent::DocumentSaved(id));
        Ok(())
    }

    /// Saves the current document to a new path.
    pub fn save_current_as(&mut self, path: impl AsRef<Path>) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.save_as(path)?;
        let id = doc.id();
        self.emit(EditorEvent::DocumentSaved(id));
        Ok(())
    }

    /// Returns the active document.
    pub fn active_document(&self) -> CoreResult<&Document> {
        self.documents.active().ok_or(CoreError::NoActiveDocument)
    }

    /// Returns a mutable reference to the active document.
    pub fn active_document_mut(&mut self) -> CoreResult<&mut Document> {
        self.documents
            .active_mut()
            .ok_or(CoreError::NoActiveDocument)
    }

    /// Returns a document by ID.
    pub fn document(&self, id: DocumentId) -> CoreResult<&Document> {
        self.documents
            .get(id)
            .ok_or(CoreError::DocumentNotFound(id))
    }

    /// Returns all open documents.
    pub fn documents(&self) -> impl Iterator<Item = &Document> {
        self.documents.iter()
    }

    // ==================== Text Editing ====================

    /// Inserts text at the current cursor position.
    pub fn insert_text(&mut self, text: &str) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.insert_at_cursor(text)?;
        self.emit_document_changed();
        Ok(())
    }

    /// Deletes the selection or character before cursor.
    pub fn delete_backward(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.delete_backward()?;
        self.emit_document_changed();
        Ok(())
    }

    /// Deletes the selection or character after cursor.
    pub fn delete_forward(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.delete_forward()?;
        self.emit_document_changed();
        Ok(())
    }

    /// Undoes the last action.
    pub fn undo(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.undo()?;
        self.emit_document_changed();
        Ok(())
    }

    /// Redoes the last undone action.
    pub fn redo(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.redo()?;
        self.emit_document_changed();
        Ok(())
    }

    // ==================== Cursor Movement ====================

    /// Moves cursor up by n lines.
    pub fn move_up(&mut self, n: usize) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.move_cursor_up(n);
        self.emit_cursor_moved();
        Ok(())
    }

    /// Moves cursor down by n lines.
    pub fn move_down(&mut self, n: usize) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.move_cursor_down(n);
        self.emit_cursor_moved();
        Ok(())
    }

    /// Moves cursor left by n characters.
    pub fn move_left(&mut self, n: usize) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.move_cursor_left(n);
        self.emit_cursor_moved();
        Ok(())
    }

    /// Moves cursor right by n characters.
    pub fn move_right(&mut self, n: usize) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.move_cursor_right(n);
        self.emit_cursor_moved();
        Ok(())
    }

    /// Moves cursor to start of line.
    pub fn move_to_line_start(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.move_to_line_start();
        self.emit_cursor_moved();
        Ok(())
    }

    /// Moves cursor to end of line.
    pub fn move_to_line_end(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.move_to_line_end();
        self.emit_cursor_moved();
        Ok(())
    }

    // ==================== Selection ====================

    /// Selects all text.
    pub fn select_all(&mut self) -> CoreResult<()> {
        let doc = self.active_document_mut()?;
        doc.select_all();
        self.emit_selection_changed();
        Ok(())
    }

    /// Copies selection to clipboard.
    pub fn copy(&mut self) -> CoreResult<()> {
        let doc = self.active_document()?;
        if let Some(text) = doc.selected_text() {
            self.clipboard = text;
        }
        Ok(())
    }

    /// Cuts selection to clipboard.
    pub fn cut(&mut self) -> CoreResult<()> {
        self.copy()?;
        let doc = self.active_document_mut()?;
        doc.delete_selection()?;
        self.emit_document_changed();
        Ok(())
    }

    /// Pastes from clipboard.
    pub fn paste(&mut self) -> CoreResult<()> {
        let text = self.clipboard.clone();
        self.insert_text(&text)?;
        Ok(())
    }

    // ==================== Mode ====================

    /// Returns the current editor mode.
    pub fn mode(&self) -> EditorMode {
        self.mode
    }

    /// Sets the editor mode.
    pub fn set_mode(&mut self, mode: EditorMode) {
        if self.mode != mode {
            self.mode = mode;
            self.emit(EditorEvent::ModeChanged(mode));
        }
    }

    /// Switches to insert mode.
    pub fn enter_insert_mode(&mut self) {
        self.set_mode(EditorMode::Insert);
    }

    /// Switches to normal mode.
    pub fn enter_normal_mode(&mut self) {
        self.set_mode(EditorMode::Normal);
    }

    // ==================== Workspace ====================

    /// Opens a workspace folder.
    pub fn open_workspace(&mut self, path: impl AsRef<Path>) -> CoreResult<()> {
        let workspace = Workspace::open(path)?;
        self.workspace = Some(workspace);
        self.emit(EditorEvent::WorkspaceOpened);
        Ok(())
    }

    /// Returns the current workspace.
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    // ==================== Configuration ====================

    /// Returns the editor configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Updates the configuration.
    pub fn set_config(&mut self, config: Config) {
        self.config = config;
        self.keymap = Keymap::from_config(&self.config);
        self.emit(EditorEvent::ConfigChanged);
    }

    /// Returns the keymap.
    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    // ==================== Lifecycle ====================

    /// Signals that the editor should quit.
    pub fn quit(&mut self) {
        self.should_quit = true;
        self.emit(EditorEvent::Quit);
    }

    /// Returns true if the editor should quit.
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// Returns true if any document has unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.documents.iter().any(|d| d.is_modified())
    }

    // ==================== Events ====================

    /// Subscribes to editor events.
    pub fn subscribe(&mut self) -> tokio::sync::broadcast::Receiver<EditorEvent> {
        self.event_bus.subscribe()
    }

    fn emit(&self, event: EditorEvent) {
        self.event_bus.emit(event);
    }

    fn emit_document_changed(&self) {
        if let Some(doc) = self.documents.active() {
            self.emit(EditorEvent::DocumentChanged(doc.id()));
        }
    }

    fn emit_cursor_moved(&self) {
        if let Some(doc) = self.documents.active() {
            self.emit(EditorEvent::CursorMoved(doc.id()));
        }
    }

    fn emit_selection_changed(&self) {
        if let Some(doc) = self.documents.active() {
            self.emit(EditorEvent::SelectionChanged(doc.id()));
        }
    }
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}
