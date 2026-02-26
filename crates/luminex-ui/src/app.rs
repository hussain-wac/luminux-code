//! Main application state and logic.
//!
//! A fully functional text editor with file browsing and editing.

use iced::keyboard;
use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, stack, text, text_editor, text_input,
    Column, Row, Space, mouse_area,
};
use iced::{Background, Border, Color, Element, Font, Length, Padding, Point, Subscription, Task, Theme};
use std::path::{Path, PathBuf};

use crate::highlighter::{detect_language, EditorHighlighter, HighlightSettings};

// ============================================================================
// Colors - Modern dark theme palette
// ============================================================================

mod colors {
    use iced::Color;

    pub const BG_DARK: Color = Color::from_rgb(0.11, 0.11, 0.13);
    pub const BG_MEDIUM: Color = Color::from_rgb(0.14, 0.14, 0.16);
    pub const BG_LIGHT: Color = Color::from_rgb(0.18, 0.18, 0.20);
    pub const BG_HOVER: Color = Color::from_rgb(0.22, 0.22, 0.25);
    pub const BG_ACTIVE: Color = Color::from_rgb(0.25, 0.25, 0.28);

    pub const TEXT_PRIMARY: Color = Color::from_rgb(0.93, 0.93, 0.93);
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.65, 0.65, 0.68);
    pub const TEXT_MUTED: Color = Color::from_rgb(0.45, 0.45, 0.48);

    pub const ACCENT: Color = Color::from_rgb(0.36, 0.54, 0.90);

    pub const BORDER: Color = Color::from_rgb(0.25, 0.25, 0.28);
}

// ============================================================================
// File Tree
// ============================================================================

#[derive(Debug, Clone)]
pub struct FileNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<FileNode>,
    pub expanded: bool,
    pub depth: u16,
}

impl FileNode {
    fn from_path(path: &Path, depth: u16) -> Option<Self> {
        let name = path.file_name()?.to_string_lossy().to_string();
        let is_dir = path.is_dir();

        Some(Self {
            name,
            path: path.to_path_buf(),
            is_dir,
            children: Vec::new(),
            expanded: false,
            depth,
        })
    }

    fn load_children(&mut self) {
        if !self.is_dir || !self.children.is_empty() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(&self.path) {
            let mut children: Vec<FileNode> = entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    // Skip hidden files
                    !e.file_name().to_string_lossy().starts_with('.')
                })
                .filter_map(|e| FileNode::from_path(&e.path(), self.depth + 1))
                .collect();

            // Sort: directories first, then alphabetically
            children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            self.children = children;
        }
    }
}

// ============================================================================
// Tab Info
// ============================================================================

struct TabInfo {
    path: Option<PathBuf>,
    name: String,
    content: text_editor::Content,
    modified: bool,
    language: String,
    // Undo/redo history
    undo_stack: Vec<String>,
    redo_stack: Vec<String>,
    last_saved_content: String,
}

impl TabInfo {
    fn new_untitled(id: usize) -> Self {
        Self {
            path: None,
            name: format!("untitled-{}.rs", id),
            content: text_editor::Content::with_text(""),
            modified: false,
            language: "rust".to_string(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_saved_content: String::new(),
        }
    }

    fn from_file(path: PathBuf, text: String) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let language = detect_language(&name);

        Self {
            path: Some(path),
            name: name.clone(),
            content: text_editor::Content::with_text(&text),
            modified: false,
            language: language.to_string(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_saved_content: text,
        }
    }

    fn save_undo_state(&mut self) {
        let current = self.content.text();
        // Only save if different from last state
        if self.undo_stack.last().map(|s| s.as_str()) != Some(&current) {
            self.undo_stack.push(current);
            // Limit undo stack size
            if self.undo_stack.len() > 100 {
                self.undo_stack.remove(0);
            }
            // Clear redo stack on new edit
            self.redo_stack.clear();
        }
    }

    fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            let current = self.content.text();
            self.redo_stack.push(current);
            self.content = text_editor::Content::with_text(&previous);
            true
        } else {
            false
        }
    }

    fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            let current = self.content.text();
            self.undo_stack.push(current);
            self.content = text_editor::Content::with_text(&next);
            true
        } else {
            false
        }
    }
}

// ============================================================================
// Application State
// ============================================================================

#[derive(Debug, Default)]
pub struct Flags {
    pub file: Option<String>,
    pub workspace: Option<String>,
}

/// Context menu state
#[derive(Debug, Clone)]
pub struct ContextMenu {
    visible: bool,
    position: Point,
    target: Option<PathBuf>,
    is_directory: bool,
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self {
            visible: false,
            position: Point::ORIGIN,
            target: None,
            is_directory: false,
        }
    }
}

/// Stores info needed to undo a file/folder deletion from the explorer.
#[derive(Debug, Clone)]
struct DeletedEntry {
    /// Original path of the deleted item.
    path: PathBuf,
    /// If the item was a file, its contents. `None` for directories.
    content: Option<Vec<u8>>,
    /// If the item was a directory, a recursive snapshot of its contents.
    children: Vec<DeletedEntry>,
    is_dir: bool,
}

impl DeletedEntry {
    /// Recursively snapshot a path before deleting it.
    fn snapshot(path: &Path) -> Option<Self> {
        let is_dir = path.is_dir();
        if is_dir {
            let mut children = Vec::new();
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if let Some(child) = Self::snapshot(&entry.path()) {
                        children.push(child);
                    }
                }
            }
            Some(Self {
                path: path.to_path_buf(),
                content: None,
                children,
                is_dir: true,
            })
        } else {
            let content = std::fs::read(path).ok();
            Some(Self {
                path: path.to_path_buf(),
                content,
                children: Vec::new(),
                is_dir: false,
            })
        }
    }

    /// Restore this entry (and its children) back to disk.
    fn restore(&self) -> Result<(), String> {
        if self.is_dir {
            std::fs::create_dir_all(&self.path)
                .map_err(|e| format!("Failed to restore directory: {}", e))?;
            for child in &self.children {
                child.restore()?;
            }
        } else if let Some(content) = &self.content {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir: {}", e))?;
            }
            std::fs::write(&self.path, content)
                .map_err(|e| format!("Failed to restore file: {}", e))?;
        }
        Ok(())
    }
}

pub struct App {
    tabs: Vec<TabInfo>,
    active_tab: usize,
    sidebar_visible: bool,
    sidebar_width: f32,
    file_tree: Option<FileNode>,
    current_folder: Option<PathBuf>,
    status_message: String,
    untitled_counter: usize,
    context_menu: ContextMenu,
    clipboard_path: Option<PathBuf>,
    clipboard_is_cut: bool,
    /// Track the last known mouse position in the sidebar for context menu placement.
    last_cursor_position: Point,
    /// Stack of deleted entries for Ctrl+Z undo in the explorer.
    deleted_stack: Vec<DeletedEntry>,
    /// Whether we are showing a delete-confirmation modal.
    confirm_delete_visible: bool,
    /// The target path pending deletion (set when confirmation is requested).
    confirm_delete_target: Option<PathBuf>,
    /// Whether the minimap panel is visible.
    minimap_visible: bool,
    /// Whether the rename modal is visible.
    rename_visible: bool,
    /// The path being renamed.
    rename_target: Option<PathBuf>,
    /// Current text in the rename input field.
    rename_input: String,
    /// Whether the editor right-click context menu is visible.
    editor_context_visible: bool,
    /// Position for the editor context menu.
    editor_context_position: Point,
    /// Which top menu bar dropdown is currently open (None = all closed).
    active_menu: Option<TopMenu>,
    /// Font size for the editor (zoom).
    font_size: f32,
    /// Whether the "Go to Line" dialog is visible.
    goto_line_visible: bool,
    /// Input text for the "Go to Line" dialog.
    goto_line_input: String,
    /// Whether the About modal is visible.
    about_visible: bool,
    /// Whether the integrated terminal is visible.
    terminal_visible: bool,
    /// Terminal output lines.
    terminal_output: Vec<String>,
    /// Terminal input buffer.
    terminal_input: String,
    /// Terminal working directory.
    terminal_cwd: PathBuf,
    /// Terminal height in pixels.
    terminal_height: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopMenu {
    File,
    Edit,
    Selection,
    View,
    Go,
    Window,
    Help,
}

#[derive(Debug, Clone)]
pub enum Message {
    // File operations
    NewFile,
    OpenFile,
    OpenFolder,
    Save,
    SaveAs,
    CloseTab(usize),
    CloseCurrentTab,
    CreateFolder,
    CreateNewFile,

    // Edit operations
    Undo,
    Redo,

    // Editor
    EditorAction(text_editor::Action),

    // Navigation
    TabSelected(usize),
    FileClicked(PathBuf),
    ToggleFolder(PathBuf),
    NextTab,
    PrevTab,

    // UI
    ToggleSidebar,

    // Context menu
    ShowContextMenu(Point, Option<PathBuf>, bool),
    HideContextMenu,
    ContextCopy,
    ContextCut,
    ContextPaste,
    ContextDelete,
    ContextNewFile,
    ContextNewFolder,
    ContextRename,

    // Mouse tracking
    MouseMoved(Point),

    // Delete confirmation modal
    ConfirmDeleteYes,
    ConfirmDeleteCancel,

    // Undo deleted file in explorer
    UndoExplorerDelete,

    // Minimap
    ToggleMinimap,

    // Rename
    RenameInputChanged(String),
    RenameConfirm,
    RenameCancel,

    // Editor context menu
    ShowEditorContextMenu,
    HideEditorContextMenu,
    EditorCut,
    EditorCopy,
    EditorPaste,
    EditorSelectAll,

    // Top menu bar
    ToggleTopMenu(TopMenu),
    CloseTopMenu,

    // Zoom
    ZoomIn,
    ZoomOut,
    ZoomReset,

    // Go to Line
    ShowGotoLine,
    GotoLineInputChanged(String),
    GotoLineConfirm,
    GotoLineCancel,

    // Selection
    SelectLine,

    // Window operations
    CloseWindow,

    // Help
    ShowAbout,
    HideAbout,

    // Terminal
    ToggleTerminal,
    TerminalInputChanged(String),
    TerminalSubmit,
    TerminalOutput(String),
    TerminalClear,

    // Async results
    FileOpened(Result<(PathBuf, String), String>),
    FolderOpened(Result<PathBuf, String>),
    FileSaved(Result<PathBuf, String>),
    FileDeleted(Result<PathBuf, String>),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut app = Self {
            tabs: vec![TabInfo::new_untitled(1)],
            active_tab: 0,
            sidebar_visible: true,
            sidebar_width: 250.0,
            file_tree: None,
            current_folder: None,
            status_message: "Ready | Ctrl+O: Open | Ctrl+S: Save | Ctrl+N: New".to_string(),
            untitled_counter: 1,
            context_menu: ContextMenu::default(),
            clipboard_path: None,
            clipboard_is_cut: false,
            last_cursor_position: Point::ORIGIN,
            deleted_stack: Vec::new(),
            confirm_delete_visible: false,
            confirm_delete_target: None,
            minimap_visible: true,
            rename_visible: false,
            rename_target: None,
            rename_input: String::new(),
            editor_context_visible: false,
            editor_context_position: Point::ORIGIN,
            active_menu: None,
            font_size: 14.0,
            goto_line_visible: false,
            goto_line_input: String::new(),
            about_visible: false,
            terminal_visible: false,
            terminal_output: vec!["Welcome to Luminex Terminal. Type commands and press Enter.".to_string()],
            terminal_input: String::new(),
            terminal_cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")),
            terminal_height: 200.0,
        };

        // Set initial content with sample Rust code
        let welcome_text = r#"// Welcome to Luminex!
// A modern text editor built with Rust

// Keyboard Shortcuts:
// Ctrl+N: New file
// Ctrl+O: Open file
// Ctrl+S: Save
// Ctrl+Shift+S: Save As
// Ctrl+Z: Undo
// Ctrl+Y: Redo
// Ctrl+W: Close tab
// Ctrl+Tab: Next tab
// Ctrl+Shift+Tab: Previous tab

fn main() {
    println!("Hello, Luminex!");

    // Try editing this text!
    let x = 42;
    let message = "Start typing...";

    if x > 10 {
        println!("{}", message);
    }
}

struct Config {
    theme: String,
    font_size: u32,
}

impl Config {
    fn new() -> Self {
        Self {
            theme: "dark".to_string(),
            font_size: 14,
        }
    }
}
"#;
        if let Some(tab) = app.tabs.get_mut(0) {
            tab.content = text_editor::Content::with_text(welcome_text);
            tab.language = "rust".to_string();
        }

        (app, Task::none())
    }

    fn title(&self) -> String {
        let name = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.name.as_str())
            .unwrap_or("Luminex");

        let modified = self
            .tabs
            .get(self.active_tab)
            .map(|t| if t.modified { " *" } else { "" })
            .unwrap_or("");

        format!("{}{} - Luminex", name, modified)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NewFile => {
                self.active_menu = None;
                self.untitled_counter += 1;
                self.tabs.push(TabInfo::new_untitled(self.untitled_counter));
                self.active_tab = self.tabs.len() - 1;
                self.status_message = "New file created".to_string();
            }

            Message::OpenFile => {
                self.active_menu = None;
                return Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new()
                            .add_filter("All Files", &["*"])
                            .add_filter("Rust", &["rs"])
                            .add_filter("Text", &["txt", "md"])
                            .add_filter("Config", &["toml", "json", "yaml", "yml"])
                            .pick_file()
                            .await;

                        match handle {
                            Some(file) => {
                                let path = file.path().to_path_buf();
                                match std::fs::read_to_string(&path) {
                                    Ok(content) => Ok((path, content)),
                                    Err(e) => Err(format!("Failed to read file: {}", e)),
                                }
                            }
                            None => Err("Cancelled".to_string()),
                        }
                    },
                    Message::FileOpened,
                );
            }

            Message::OpenFolder => {
                self.active_menu = None;
                return Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new().pick_folder().await;

                        match handle {
                            Some(folder) => Ok(folder.path().to_path_buf()),
                            None => Err("Cancelled".to_string()),
                        }
                    },
                    Message::FolderOpened,
                );
            }

            Message::Save => {
                self.active_menu = None;
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(path) = &tab.path {
                        let path = path.clone();
                        let content = tab.content.text();
                        return Task::perform(
                            async move {
                                match std::fs::write(&path, content) {
                                    Ok(_) => Ok(path),
                                    Err(e) => Err(format!("Failed to save: {}", e)),
                                }
                            },
                            Message::FileSaved,
                        );
                    } else {
                        return self.update(Message::SaveAs);
                    }
                }
            }

            Message::SaveAs => {
                self.active_menu = None;
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    let content = tab.content.text();
                    let default_name = tab.name.clone();
                    return Task::perform(
                        async move {
                            let handle = rfd::AsyncFileDialog::new()
                                .set_file_name(&default_name)
                                .save_file()
                                .await;

                            match handle {
                                Some(file) => {
                                    let path = file.path().to_path_buf();
                                    match std::fs::write(&path, content) {
                                        Ok(_) => Ok(path),
                                        Err(e) => Err(format!("Failed to save: {}", e)),
                                    }
                                }
                                None => Err("Cancelled".to_string()),
                            }
                        },
                        Message::FileSaved,
                    );
                }
            }

            Message::CloseTab(idx) => {
                if self.tabs.len() > 1 && idx < self.tabs.len() {
                    self.tabs.remove(idx);
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    } else if idx < self.active_tab {
                        self.active_tab = self.active_tab.saturating_sub(1);
                    }
                    self.status_message = "Tab closed".to_string();
                }
            }

            Message::CloseCurrentTab => {
                self.active_menu = None;
                let idx = self.active_tab;
                if self.tabs.len() > 1 && idx < self.tabs.len() {
                    self.tabs.remove(idx);
                    if self.active_tab >= self.tabs.len() {
                        self.active_tab = self.tabs.len() - 1;
                    }
                    self.status_message = "Tab closed".to_string();
                }
            }

            Message::CreateFolder => {
                if let Some(current_folder) = &self.current_folder {
                    let new_folder_path = current_folder.join("New Folder");
                    let mut counter = 1;
                    let mut final_path = new_folder_path.clone();

                    // Find unique name
                    while final_path.exists() {
                        final_path = current_folder.join(format!("New Folder {}", counter));
                        counter += 1;
                    }

                    if let Err(e) = std::fs::create_dir(&final_path) {
                        self.status_message = format!("Failed to create folder: {}", e);
                    } else {
                        // Refresh file tree
                        if let Some(mut tree) = FileNode::from_path(current_folder, 0) {
                            tree.expanded = true;
                            tree.load_children();
                            self.file_tree = Some(tree);
                        }
                        self.status_message = format!("Created folder: {}", final_path.file_name().unwrap_or_default().to_string_lossy());
                    }
                } else {
                    self.status_message = "Open a folder first".to_string();
                }
            }

            Message::Undo => {
                self.active_menu = None;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if tab.undo() {
                        self.status_message = "Undo".to_string();
                    } else {
                        self.status_message = "Nothing to undo".to_string();
                    }
                }
            }

            Message::Redo => {
                self.active_menu = None;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if tab.redo() {
                        self.status_message = "Redo".to_string();
                    } else {
                        self.status_message = "Nothing to redo".to_string();
                    }
                }
            }

            Message::EditorAction(action) => {
                // Close any open context menus when the editor is interacted with
                self.context_menu.visible = false;
                self.editor_context_visible = false;
                self.active_menu = None;

                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let is_edit = action.is_edit();

                    // Save state before edit for undo
                    if is_edit {
                        tab.save_undo_state();
                    }

                    tab.content.perform(action);

                    if is_edit {
                        tab.modified = true;
                    }
                }
            }

            Message::TabSelected(idx) => {
                self.context_menu.visible = false;
                self.editor_context_visible = false;
                self.active_menu = None;
                if idx < self.tabs.len() {
                    self.active_tab = idx;
                    let name = self.tabs[idx].name.clone();
                    self.status_message = format!("Editing: {}", name);
                }
            }

            Message::NextTab => {
                self.active_menu = None;
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                    let name = self.tabs[self.active_tab].name.clone();
                    self.status_message = format!("Switched to: {}", name);
                }
            }

            Message::PrevTab => {
                self.active_menu = None;
                if !self.tabs.is_empty() {
                    self.active_tab = if self.active_tab == 0 {
                        self.tabs.len() - 1
                    } else {
                        self.active_tab - 1
                    };
                    let name = self.tabs[self.active_tab].name.clone();
                    self.status_message = format!("Switched to: {}", name);
                }
            }

            Message::FileClicked(path) => {
                self.context_menu.visible = false;
                self.editor_context_visible = false;
                self.active_menu = None;
                if path.is_file() {
                    // Check if already open
                    for (idx, tab) in self.tabs.iter().enumerate() {
                        if tab.path.as_ref() == Some(&path) {
                            self.active_tab = idx;
                            self.status_message = format!("Switched to: {}", tab.name);
                            return Task::none();
                        }
                    }

                    let path_clone = path.clone();
                    return Task::perform(
                        async move {
                            match std::fs::read_to_string(&path_clone) {
                                Ok(content) => Ok((path_clone, content)),
                                Err(e) => Err(format!("Failed to read file: {}", e)),
                            }
                        },
                        Message::FileOpened,
                    );
                }
            }

            Message::ToggleFolder(path) => {
                self.context_menu.visible = false;
                self.editor_context_visible = false;
                self.active_menu = None;
                if let Some(tree) = &mut self.file_tree {
                    Self::toggle_folder_recursive(tree, &path);
                }
            }

            Message::ToggleSidebar => {
                self.context_menu.visible = false;
                self.editor_context_visible = false;
                self.active_menu = None;
                self.sidebar_visible = !self.sidebar_visible;
            }

            Message::FileOpened(result) => match result {
                Ok((path, content)) => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    self.tabs.push(TabInfo::from_file(path, content));
                    self.active_tab = self.tabs.len() - 1;
                    self.status_message = format!("Opened: {}", name);
                }
                Err(e) => {
                    if e != "Cancelled" {
                        self.status_message = format!("Error: {}", e);
                    }
                }
            },

            Message::FolderOpened(result) => match result {
                Ok(path) => {
                    let folder_name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "folder".to_string());

                    if let Some(mut tree) = FileNode::from_path(&path, 0) {
                        tree.expanded = true;
                        tree.load_children();
                        self.file_tree = Some(tree);
                    }
                    self.current_folder = Some(path);
                    self.status_message = format!("Opened folder: {}", folder_name);
                }
                Err(e) => {
                    if e != "Cancelled" {
                        self.status_message = format!("Error: {}", e);
                    }
                }
            },

            Message::FileSaved(result) => match result {
                Ok(path) => {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "file".to_string());

                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        tab.path = Some(path);
                        tab.name = name.clone();
                        tab.modified = false;
                    }
                    self.status_message = format!("Saved: {}", name);
                }
                Err(e) => {
                    if e != "Cancelled" {
                        self.status_message = format!("Error: {}", e);
                    }
                }
            },

            Message::FileDeleted(result) => match result {
                Ok(path) => {
                    let name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "item".to_string());
                    self.refresh_file_tree();
                    self.status_message = format!("Deleted: {} (Ctrl+Z to undo)", name);
                }
                Err(e) => {
                    // Deletion failed, remove the snapshot we took
                    self.deleted_stack.pop();
                    self.status_message = format!("Delete failed: {}", e);
                }
            },

            Message::CreateNewFile => {
                if let Some(current_folder) = &self.current_folder {
                    let new_file_path = current_folder.join("untitled.txt");
                    let mut counter = 1;
                    let mut final_path = new_file_path.clone();

                    while final_path.exists() {
                        final_path = current_folder.join(format!("untitled_{}.txt", counter));
                        counter += 1;
                    }

                    if let Err(e) = std::fs::write(&final_path, "") {
                        self.status_message = format!("Failed to create file: {}", e);
                    } else {
                        self.refresh_file_tree();
                        self.status_message = format!("Created: {}", final_path.file_name().unwrap_or_default().to_string_lossy());
                        // Open the new file
                        return self.update(Message::FileClicked(final_path));
                    }
                } else {
                    self.status_message = "Open a folder first".to_string();
                }
            }

            Message::MouseMoved(point) => {
                self.last_cursor_position = point;
            }

            // Context menu messages
            Message::ShowContextMenu(_position, target, is_directory) => {
                self.context_menu = ContextMenu {
                    visible: true,
                    position: self.last_cursor_position,
                    target,
                    is_directory,
                };
            }

            Message::HideContextMenu => {
                self.context_menu.visible = false;
            }

            Message::ContextCopy => {
                if let Some(target) = &self.context_menu.target {
                    self.clipboard_path = Some(target.clone());
                    self.clipboard_is_cut = false;
                    self.status_message = format!("Copied: {}", target.file_name().unwrap_or_default().to_string_lossy());
                }
                self.context_menu.visible = false;
            }

            Message::ContextCut => {
                if let Some(target) = &self.context_menu.target {
                    self.clipboard_path = Some(target.clone());
                    self.clipboard_is_cut = true;
                    self.status_message = format!("Cut: {}", target.file_name().unwrap_or_default().to_string_lossy());
                }
                self.context_menu.visible = false;
            }

            Message::ContextPaste => {
                self.context_menu.visible = false;
                if let Some(src_path) = &self.clipboard_path.clone() {
                    // Determine destination folder
                    let dest_folder = if let Some(target) = &self.context_menu.target {
                        if target.is_dir() {
                            target.clone()
                        } else {
                            target.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| self.current_folder.clone().unwrap_or_default())
                        }
                    } else {
                        self.current_folder.clone().unwrap_or_default()
                    };

                    let file_name = src_path.file_name().unwrap_or_default();
                    let mut dest_path = dest_folder.join(file_name);

                    // Handle name conflicts
                    let mut counter = 1;
                    while dest_path.exists() && dest_path != *src_path {
                        let stem = src_path.file_stem().unwrap_or_default().to_string_lossy();
                        let ext = src_path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
                        dest_path = dest_folder.join(format!("{}_copy{}{}", stem, counter, ext));
                        counter += 1;
                    }

                    let result = if self.clipboard_is_cut {
                        std::fs::rename(src_path, &dest_path)
                    } else if src_path.is_dir() {
                        Self::copy_dir_recursive(src_path, &dest_path)
                    } else {
                        std::fs::copy(src_path, &dest_path).map(|_| ())
                    };

                    match result {
                        Ok(_) => {
                            if self.clipboard_is_cut {
                                self.clipboard_path = None;
                            }
                            self.refresh_file_tree();
                            self.status_message = format!("Pasted: {}", dest_path.file_name().unwrap_or_default().to_string_lossy());
                        }
                        Err(e) => {
                            self.status_message = format!("Paste failed: {}", e);
                        }
                    }
                } else {
                    self.status_message = "Nothing to paste".to_string();
                }
            }

            Message::ContextDelete => {
                self.context_menu.visible = false;
                if let Some(target) = self.context_menu.target.clone() {
                    // Show confirmation modal instead of deleting immediately
                    self.confirm_delete_target = Some(target);
                    self.confirm_delete_visible = true;
                }
            }

            Message::ConfirmDeleteCancel => {
                self.confirm_delete_visible = false;
                self.confirm_delete_target = None;
                self.status_message = "Delete cancelled".to_string();
            }

            Message::ConfirmDeleteYes => {
                self.confirm_delete_visible = false;
                if let Some(target) = self.confirm_delete_target.take() {
                    // Snapshot before deletion for undo
                    if let Some(snapshot) = DeletedEntry::snapshot(&target) {
                        self.deleted_stack.push(snapshot);
                        // Keep stack bounded
                        if self.deleted_stack.len() > 50 {
                            self.deleted_stack.remove(0);
                        }
                    }

                    let target_clone = target.clone();
                    return Task::perform(
                        async move {
                            let result = if target_clone.is_dir() {
                                std::fs::remove_dir_all(&target_clone)
                            } else {
                                std::fs::remove_file(&target_clone)
                            };
                            match result {
                                Ok(_) => Ok(target_clone),
                                Err(e) => Err(e.to_string()),
                            }
                        },
                        Message::FileDeleted,
                    );
                }
            }

            Message::UndoExplorerDelete => {
                if let Some(entry) = self.deleted_stack.pop() {
                    let name = entry.path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "item".to_string());
                    match entry.restore() {
                        Ok(_) => {
                            self.refresh_file_tree();
                            self.status_message = format!("Restored: {}", name);
                        }
                        Err(e) => {
                            self.status_message = format!("Restore failed: {}", e);
                            // Put it back since restore failed
                            // (entry is consumed, can't push back easily)
                        }
                    }
                } else {
                    // No deleted explorer entry, fall through to editor undo
                    if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                        if tab.undo() {
                            self.status_message = "Undo".to_string();
                        } else {
                            self.status_message = "Nothing to undo".to_string();
                        }
                    }
                }
            }

            Message::ContextNewFile => {
                self.context_menu.visible = false;
                // Determine target folder
                let target_folder = if let Some(target) = &self.context_menu.target {
                    if target.is_dir() {
                        target.clone()
                    } else {
                        target.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| self.current_folder.clone().unwrap_or_default())
                    }
                } else {
                    self.current_folder.clone().unwrap_or_default()
                };

                let mut new_file_path = target_folder.join("untitled.txt");
                let mut counter = 1;
                while new_file_path.exists() {
                    new_file_path = target_folder.join(format!("untitled_{}.txt", counter));
                    counter += 1;
                }

                if let Err(e) = std::fs::write(&new_file_path, "") {
                    self.status_message = format!("Failed to create file: {}", e);
                } else {
                    self.refresh_file_tree();
                    self.status_message = format!("Created: {}", new_file_path.file_name().unwrap_or_default().to_string_lossy());
                    return self.update(Message::FileClicked(new_file_path));
                }
            }

            Message::ContextNewFolder => {
                self.context_menu.visible = false;
                // Determine target folder
                let target_folder = if let Some(target) = &self.context_menu.target {
                    if target.is_dir() {
                        target.clone()
                    } else {
                        target.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| self.current_folder.clone().unwrap_or_default())
                    }
                } else {
                    self.current_folder.clone().unwrap_or_default()
                };

                let mut new_folder_path = target_folder.join("New Folder");
                let mut counter = 1;
                while new_folder_path.exists() {
                    new_folder_path = target_folder.join(format!("New Folder {}", counter));
                    counter += 1;
                }

                if let Err(e) = std::fs::create_dir(&new_folder_path) {
                    self.status_message = format!("Failed to create folder: {}", e);
                } else {
                    self.refresh_file_tree();
                    self.status_message = format!("Created: {}", new_folder_path.file_name().unwrap_or_default().to_string_lossy());
                }
            }

            Message::ContextRename => {
                self.context_menu.visible = false;
                if let Some(target) = self.context_menu.target.clone() {
                    let current_name = target.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.rename_target = Some(target);
                    self.rename_input = current_name;
                    self.rename_visible = true;
                }
            }

            Message::RenameInputChanged(value) => {
                self.rename_input = value;
            }

            Message::RenameCancel => {
                self.rename_visible = false;
                self.rename_target = None;
                self.rename_input.clear();
                self.status_message = "Rename cancelled".to_string();
            }

            Message::RenameConfirm => {
                self.rename_visible = false;
                let new_name = self.rename_input.trim().to_string();
                if let Some(target) = self.rename_target.take() {
                    if new_name.is_empty() {
                        self.status_message = "Rename failed: name cannot be empty".to_string();
                    } else if let Some(parent) = target.parent() {
                        let new_path = parent.join(&new_name);
                        if new_path == target {
                            self.status_message = "Name unchanged".to_string();
                        } else if new_path.exists() {
                            self.status_message = format!("Rename failed: \"{}\" already exists", new_name);
                        } else {
                            match std::fs::rename(&target, &new_path) {
                                Ok(_) => {
                                    // Update any open tab that references this path
                                    for tab in &mut self.tabs {
                                        if tab.path.as_ref() == Some(&target) {
                                            tab.path = Some(new_path.clone());
                                            tab.name = new_name.clone();
                                            tab.language = detect_language(&new_name).to_string();
                                        }
                                    }
                                    self.refresh_file_tree();
                                    self.status_message = format!("Renamed to: {}", new_name);
                                }
                                Err(e) => {
                                    self.status_message = format!("Rename failed: {}", e);
                                }
                            }
                        }
                    }
                }
                self.rename_input.clear();
            }

            Message::ToggleMinimap => {
                self.minimap_visible = !self.minimap_visible;
            }

            // Top menu bar
            Message::ToggleTopMenu(menu) => {
                if self.active_menu == Some(menu) {
                    self.active_menu = None;
                } else {
                    self.active_menu = Some(menu);
                }
            }

            Message::CloseTopMenu => {
                self.active_menu = None;
                self.editor_context_visible = false;
                self.goto_line_visible = false;
                self.about_visible = false;
            }

            // Editor context menu
            Message::ShowEditorContextMenu => {
                self.editor_context_visible = true;
                self.editor_context_position = self.last_cursor_position;
                // Close file explorer context menu if open
                self.context_menu.visible = false;
            }

            Message::HideEditorContextMenu => {
                self.editor_context_visible = false;
            }

            Message::EditorCut => {
                self.editor_context_visible = false;
                self.active_menu = None;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let Some(selected) = tab.content.selection() {
                        tab.save_undo_state();
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&selected);
                        }
                        tab.content.perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                        tab.modified = true;
                        self.status_message = "Cut".to_string();
                    }
                }
            }

            Message::EditorCopy => {
                self.editor_context_visible = false;
                self.active_menu = None;
                if let Some(tab) = self.tabs.get(self.active_tab) {
                    if let Some(selected) = tab.content.selection() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&selected);
                        }
                        self.status_message = "Copied".to_string();
                    } else {
                        self.status_message = "Nothing selected".to_string();
                    }
                }
            }

            Message::EditorPaste => {
                self.editor_context_visible = false;
                self.active_menu = None;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(clip_text) = clipboard.get_text() {
                            tab.save_undo_state();
                            tab.content.perform(text_editor::Action::Edit(
                                text_editor::Edit::Paste(std::sync::Arc::new(clip_text))
                            ));
                            tab.modified = true;
                            self.status_message = "Pasted".to_string();
                        }
                    }
                }
            }

            Message::EditorSelectAll => {
                self.editor_context_visible = false;
                self.active_menu = None;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.content.perform(text_editor::Action::SelectAll);
                    self.status_message = "Selected all".to_string();
                }
            }

            // Zoom
            Message::ZoomIn => {
                self.active_menu = None;
                self.font_size = (self.font_size + 2.0).min(40.0);
                self.status_message = format!("Zoom: {}px", self.font_size);
            }
            Message::ZoomOut => {
                self.active_menu = None;
                self.font_size = (self.font_size - 2.0).max(8.0);
                self.status_message = format!("Zoom: {}px", self.font_size);
            }
            Message::ZoomReset => {
                self.active_menu = None;
                self.font_size = 14.0;
                self.status_message = "Zoom reset to 14px".to_string();
            }

            // Go to Line
            Message::ShowGotoLine => {
                self.active_menu = None;
                self.goto_line_visible = true;
                self.goto_line_input = String::new();
            }
            Message::GotoLineInputChanged(val) => {
                self.goto_line_input = val;
            }
            Message::GotoLineConfirm => {
                self.goto_line_visible = false;
                if let Ok(line_num) = self.goto_line_input.trim().parse::<usize>() {
                    if line_num > 0 {
                        if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                            // Move to document start first, then move down line_num-1 lines
                            tab.content.perform(text_editor::Action::Move(
                                text_editor::Motion::DocumentStart,
                            ));
                            for _ in 0..line_num.saturating_sub(1) {
                                tab.content.perform(text_editor::Action::Move(
                                    text_editor::Motion::Down,
                                ));
                            }
                            self.status_message = format!("Go to line {}", line_num);
                        }
                    }
                }
            }
            Message::GotoLineCancel => {
                self.goto_line_visible = false;
            }

            // Select Line
            Message::SelectLine => {
                self.active_menu = None;
                self.editor_context_visible = false;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.content.perform(text_editor::Action::SelectLine);
                    self.status_message = "Line selected".to_string();
                }
            }

            // Window operations
            Message::CloseWindow => {
                self.active_menu = None;
                return iced::exit();
            }

            // Help
            Message::ShowAbout => {
                self.active_menu = None;
                self.about_visible = true;
            }
            Message::HideAbout => {
                self.about_visible = false;
            }

            // Terminal
            Message::ToggleTerminal => {
                self.active_menu = None;
                self.terminal_visible = !self.terminal_visible;
                if self.terminal_visible {
                    // Set terminal cwd to project folder if available
                    if let Some(folder) = &self.current_folder {
                        self.terminal_cwd = folder.clone();
                    }
                    self.status_message = "Terminal opened".to_string();
                } else {
                    self.status_message = "Terminal closed".to_string();
                }
            }
            Message::TerminalInputChanged(val) => {
                self.terminal_input = val;
            }
            Message::TerminalSubmit => {
                let cmd = self.terminal_input.trim().to_string();
                if cmd.is_empty() {
                    return Task::none();
                }
                self.terminal_input.clear();

                // Display the command
                let prompt = format!("$ {}", cmd);
                self.terminal_output.push(prompt);

                // Handle built-in commands
                if cmd == "clear" || cmd == "cls" {
                    self.terminal_output.clear();
                    return Task::none();
                }

                if cmd.starts_with("cd ") {
                    let dir = cmd[3..].trim();
                    let new_path = if dir.starts_with('/') {
                        PathBuf::from(dir)
                    } else if dir == "~" {
                        std::env::var("HOME").map(PathBuf::from).unwrap_or(self.terminal_cwd.clone())
                    } else {
                        self.terminal_cwd.join(dir)
                    };
                    if new_path.is_dir() {
                        self.terminal_cwd = new_path.canonicalize().unwrap_or(new_path);
                    } else {
                        self.terminal_output.push(format!("cd: {}: No such directory", dir));
                    }
                    return Task::none();
                }

                // Execute external command
                let cwd = self.terminal_cwd.clone();
                return Task::perform(
                    async move {
                        let output = tokio::process::Command::new("sh")
                            .arg("-c")
                            .arg(&cmd)
                            .current_dir(&cwd)
                            .output()
                            .await;

                        match output {
                            Ok(out) => {
                                let mut result = String::new();
                                let stdout = String::from_utf8_lossy(&out.stdout);
                                let stderr = String::from_utf8_lossy(&out.stderr);
                                if !stdout.is_empty() {
                                    result.push_str(&stdout);
                                }
                                if !stderr.is_empty() {
                                    if !result.is_empty() {
                                        result.push('\n');
                                    }
                                    result.push_str(&stderr);
                                }
                                if result.is_empty() {
                                    result = "(no output)".to_string();
                                }
                                result
                            }
                            Err(e) => format!("Error: {}", e),
                        }
                    },
                    Message::TerminalOutput,
                );
            }
            Message::TerminalOutput(output) => {
                // Split output into lines and add to terminal
                for line in output.lines() {
                    self.terminal_output.push(line.to_string());
                }
                // Limit terminal output buffer
                if self.terminal_output.len() > 1000 {
                    let drain_count = self.terminal_output.len() - 1000;
                    self.terminal_output.drain(0..drain_count);
                }
            }
            Message::TerminalClear => {
                self.terminal_output.clear();
            }

        }
        Task::none()
    }

    fn refresh_file_tree(&mut self) {
        if let Some(current_folder) = &self.current_folder {
            // Collect the set of currently expanded paths before rebuilding
            let mut expanded_paths = std::collections::HashSet::new();
            if let Some(old_tree) = &self.file_tree {
                Self::collect_expanded_paths(old_tree, &mut expanded_paths);
            }

            if let Some(mut tree) = FileNode::from_path(current_folder, 0) {
                tree.expanded = true;
                tree.load_children();
                // Re-expand all folders that were expanded before
                Self::restore_expanded_state(&mut tree, &expanded_paths);
                self.file_tree = Some(tree);
            }
        }
    }

    fn collect_expanded_paths(node: &FileNode, set: &mut std::collections::HashSet<PathBuf>) {
        if node.expanded {
            set.insert(node.path.clone());
            for child in &node.children {
                Self::collect_expanded_paths(child, set);
            }
        }
    }

    fn restore_expanded_state(node: &mut FileNode, expanded: &std::collections::HashSet<PathBuf>) {
        if node.is_dir && expanded.contains(&node.path) {
            node.expanded = true;
            node.load_children();
            for child in &mut node.children {
                Self::restore_expanded_state(child, expanded);
            }
        }
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if ty.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
        Ok(())
    }

    fn subscription(&self) -> Subscription<Message> {
        keyboard::on_key_press(|key, modifiers| {
            if modifiers.control() {
                let char_key = match &key {
                    keyboard::Key::Character(c) => Some(c.to_lowercase()),
                    _ => None,
                };

                if let Some(c) = char_key {
                    match c.as_str() {
                        "n" => return Some(Message::NewFile),
                        "o" => return Some(Message::OpenFile),
                        "s" => {
                            if modifiers.shift() {
                                return Some(Message::SaveAs);
                            } else {
                                return Some(Message::Save);
                            }
                        }
                        "w" => return Some(Message::CloseCurrentTab),
                        "z" => {
                            if modifiers.shift() {
                                return Some(Message::Redo);
                            } else {
                                return Some(Message::Undo);
                            }
                        }
                        "y" => return Some(Message::Redo),
                        "g" => return Some(Message::ShowGotoLine),
                        "q" => return Some(Message::CloseWindow),
                        "=" | "+" => return Some(Message::ZoomIn),
                        "-" => return Some(Message::ZoomOut),
                        "0" => return Some(Message::ZoomReset),
                        "`" => return Some(Message::ToggleTerminal),
                        _ => {}
                    }
                }

                // Tab switching
                if matches!(key, keyboard::Key::Named(keyboard::key::Named::Tab)) {
                    if modifiers.shift() {
                        return Some(Message::PrevTab);
                    } else {
                        return Some(Message::NextTab);
                    }
                }
            }

            // Escape to close modals/menus
            if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                return Some(Message::CloseTopMenu);
            }

            None
        })
    }

    fn toggle_folder_recursive(node: &mut FileNode, target: &Path) {
        if node.path == target {
            node.expanded = !node.expanded;
            if node.expanded {
                node.load_children();
            }
            return;
        }
        for child in &mut node.children {
            Self::toggle_folder_recursive(child, target);
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let content = column![
            self.view_toolbar(),
            row![
                if self.sidebar_visible {
                    self.view_sidebar()
                } else {
                    container(Space::new(0, 0)).into()
                },
                self.view_main_area(),
            ]
            .height(Length::Fill),
            self.view_status_bar(),
        ];

        let main_view: Element<'_, Message> = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_DARK)),
                ..Default::default()
            })
            .into();

        // Track mouse movement globally for context menu positioning
        let tracked_view: Element<'_, Message> = mouse_area(main_view)
            .on_move(Message::MouseMoved)
            .into();

        // If delete confirmation modal is visible, show it
        if self.confirm_delete_visible {
            stack![
                tracked_view,
                // Dim overlay
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::ConfirmDeleteCancel),
                self.view_confirm_delete_modal(),
            ]
            .into()
        } else if self.rename_visible {
            stack![
                tracked_view,
                // Dim overlay
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::RenameCancel),
                self.view_rename_modal(),
            ]
            .into()
        } else if self.context_menu.visible {
            stack![
                // Click-away layer to close menu
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .on_press(Message::HideContextMenu),
                tracked_view,
                self.view_context_menu(),
            ]
            .into()
        } else if self.editor_context_visible {
            stack![
                // Click-away layer to close editor context menu
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .on_press(Message::HideEditorContextMenu),
                tracked_view,
                self.view_editor_context_menu(),
            ]
            .into()
        } else if self.goto_line_visible {
            stack![
                tracked_view,
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::GotoLineCancel),
                self.view_goto_line_modal(),
            ]
            .into()
        } else if self.about_visible {
            stack![
                tracked_view,
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.5))),
                            ..Default::default()
                        })
                )
                .on_press(Message::HideAbout),
                self.view_about_modal(),
            ]
            .into()
        } else if self.active_menu.is_some() {
            stack![
                // Click-away layer to close dropdown
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .on_press(Message::CloseTopMenu),
                tracked_view,
                self.view_menu_dropdown(),
            ]
            .into()
        } else {
            tracked_view
        }
    }

    fn view_context_menu(&self) -> Element<'_, Message> {
        let menu_btn_style = |_: &Theme, status: button::Status| -> button::Style {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => colors::BG_MEDIUM,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        };

        let separator: Element<'_, Message> = container(Space::new(Length::Fill, 1))
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into();

        // Build menu items based on context
        let mut items: Vec<Element<'_, Message>> = Vec::new();

        if self.context_menu.target.is_some() {
            // Items for when we have a specific file/folder selected
            items.push(
                button(text("New File").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFile)
                    .into()
            );
            items.push(
                button(text("New Folder").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFolder)
                    .into()
            );
            items.push(separator);
            items.push(
                button(text("Copy").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextCopy)
                    .into()
            );
            items.push(
                button(text("Cut").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextCut)
                    .into()
            );
            if self.clipboard_path.is_some() {
                items.push(
                    button(text("Paste").size(12).color(colors::TEXT_PRIMARY))
                        .width(Length::Fill)
                        .padding(Padding::from([6, 12]))
                        .style(menu_btn_style)
                        .on_press(Message::ContextPaste)
                        .into()
                );
            }
            items.push(
                container(Space::new(Length::Fill, 1))
                    .style(|_| container::Style {
                        background: Some(Background::Color(colors::BORDER)),
                        ..Default::default()
                    })
                    .into()
            );
            items.push(
                button(text("Rename").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextRename)
                    .into()
            );
            items.push(
                button(text("Delete").size(12).color(Color::from_rgb(0.9, 0.4, 0.4)))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextDelete)
                    .into()
            );
        } else {
            // Items for empty space (no specific target)
            items.push(
                button(text("New File").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFile)
                    .into()
            );
            items.push(
                button(text("New Folder").size(12).color(colors::TEXT_PRIMARY))
                    .width(Length::Fill)
                    .padding(Padding::from([6, 12]))
                    .style(menu_btn_style)
                    .on_press(Message::ContextNewFolder)
                    .into()
            );
            if self.clipboard_path.is_some() {
                items.push(
                    container(Space::new(Length::Fill, 1))
                        .style(|_| container::Style {
                            background: Some(Background::Color(colors::BORDER)),
                            ..Default::default()
                        })
                        .into()
                );
                items.push(
                    button(text("Paste").size(12).color(colors::TEXT_PRIMARY))
                        .width(Length::Fill)
                        .padding(Padding::from([6, 12]))
                        .style(menu_btn_style)
                        .on_press(Message::ContextPaste)
                        .into()
                );
            }
        }

        let menu_content = Column::with_children(items).width(Length::Fixed(150.0));

        // Position the context menu at the cursor position
        let x = self.context_menu.position.x;
        let y = self.context_menu.position.y;

        let menu_box = container(menu_content)
            .padding(4)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        // Use a row/column with fixed-size spacers to push the menu to the cursor position
        column![
            Space::with_height(Length::Fixed(y)),
            row![
                Space::with_width(Length::Fixed(x)),
                menu_box,
            ],
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_confirm_delete_modal(&self) -> Element<'_, Message> {
        let target_name = self.confirm_delete_target
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "this item".to_string());

        let is_dir = self.confirm_delete_target
            .as_ref()
            .map(|p| p.is_dir())
            .unwrap_or(false);

        let item_type = if is_dir { "folder" } else { "file" };

        let modal_content = column![
            text("Confirm Delete").size(16).color(colors::TEXT_PRIMARY),
            Space::with_height(12),
            text(format!("Are you sure you want to delete the {} \"{}\"?", item_type, target_name))
                .size(13)
                .color(colors::TEXT_SECONDARY),
            Space::with_height(4),
            text("You can undo this with Ctrl+Z.")
                .size(11)
                .color(colors::TEXT_MUTED),
            Space::with_height(16),
            row![
                button(
                    text("Cancel").size(13).color(colors::TEXT_PRIMARY)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_LIGHT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: colors::TEXT_PRIMARY,
                        border: Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::ConfirmDeleteCancel),
                Space::with_width(12),
                button(
                    text("Delete").size(13).color(Color::from_rgb(1.0, 1.0, 1.0))
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => Color::from_rgb(0.85, 0.25, 0.25),
                        _ => Color::from_rgb(0.75, 0.22, 0.22),
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::ConfirmDeleteYes),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(24)
        .width(Length::Fixed(380.0));

        // Center the modal on screen
        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn view_rename_modal(&self) -> Element<'_, Message> {
        let original_name = self.rename_target
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let modal_content = column![
            text("Rename").size(16).color(colors::TEXT_PRIMARY),
            Space::with_height(12),
            text(format!("Renaming: \"{}\"", original_name))
                .size(12)
                .color(colors::TEXT_MUTED),
            Space::with_height(8),
            text_input("Enter new name...", &self.rename_input)
                .on_input(Message::RenameInputChanged)
                .on_submit(Message::RenameConfirm)
                .padding(Padding::from([8, 12]))
                .size(13),
            Space::with_height(16),
            row![
                button(
                    text("Cancel").size(13).color(colors::TEXT_PRIMARY)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_LIGHT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: colors::TEXT_PRIMARY,
                        border: Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::RenameCancel),
                Space::with_width(12),
                button(
                    text("Rename").size(13).color(Color::WHITE)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => Color::from_rgb(0.40, 0.58, 0.95),
                        _ => colors::ACCENT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::RenameConfirm),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(24)
        .width(Length::Fixed(380.0));

        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn view_goto_line_modal(&self) -> Element<'_, Message> {
        let modal_content = column![
            text("Go to Line").size(16).color(colors::TEXT_PRIMARY),
            Space::with_height(12),
            text_input("Line number...", &self.goto_line_input)
                .on_input(Message::GotoLineInputChanged)
                .on_submit(Message::GotoLineConfirm)
                .padding(Padding::from([8, 12]))
                .size(13),
            Space::with_height(16),
            row![
                button(
                    text("Cancel").size(13).color(colors::TEXT_PRIMARY)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_LIGHT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: colors::TEXT_PRIMARY,
                        border: Border {
                            color: colors::BORDER,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::GotoLineCancel),
                Space::with_width(12),
                button(
                    text("Go").size(13).color(Color::WHITE)
                )
                .padding(Padding::from([8, 20]))
                .style(|_: &Theme, status: button::Status| {
                    let bg = match status {
                        button::Status::Hovered => Color::from_rgb(0.40, 0.58, 0.95),
                        _ => colors::ACCENT,
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
                .on_press(Message::GotoLineConfirm),
            ]
            .align_y(iced::Alignment::Center),
        ]
        .padding(24)
        .width(Length::Fixed(320.0));

        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    fn view_about_modal(&self) -> Element<'_, Message> {
        let modal_content = column![
            text("Luminex").size(20).color(colors::ACCENT),
            Space::with_height(8),
            text("A modern text editor built with Rust & Iced")
                .size(13)
                .color(colors::TEXT_SECONDARY),
            Space::with_height(12),
            text("Version 0.1.0").size(12).color(colors::TEXT_MUTED),
            Space::with_height(20),
            button(
                text("Close").size(13).color(Color::WHITE)
            )
            .padding(Padding::from([8, 24]))
            .style(|_: &Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => Color::from_rgb(0.40, 0.58, 0.95),
                    _ => colors::ACCENT,
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::HideAbout),
        ]
        .padding(24)
        .width(Length::Fixed(340.0))
        .align_x(iced::Alignment::Center);

        container(
            container(modal_content)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                })
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    // ========================================================================
    // Menu Bar (Zed-style top menu)
    // ========================================================================

    fn view_toolbar(&self) -> Element<'_, Message> {
        let menus = [
            TopMenu::File,
            TopMenu::Edit,
            TopMenu::Selection,
            TopMenu::View,
            TopMenu::Go,
            TopMenu::Window,
            TopMenu::Help,
        ];

        let mut menu_items: Vec<Element<'_, Message>> = Vec::new();

        for menu in menus {
            let label = match menu {
                TopMenu::File => "File",
                TopMenu::Edit => "Edit",
                TopMenu::Selection => "Selection",
                TopMenu::View => "View",
                TopMenu::Go => "Go",
                TopMenu::Window => "Window",
                TopMenu::Help => "Help",
            };

            let is_active = self.active_menu == Some(menu);

            let menu_btn = button(
                text(label).size(12).color(if is_active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                }),
            )
            .padding(Padding::from([6, 10]))
            .style(move |_: &Theme, status: button::Status| {
                let bg = if is_active {
                    colors::BG_ACTIVE
                } else {
                    match status {
                        button::Status::Hovered => colors::BG_HOVER,
                        _ => colors::BG_MEDIUM,
                    }
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: colors::TEXT_PRIMARY,
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::ToggleTopMenu(menu));

            menu_items.push(menu_btn.into());
        }

        menu_items.push(horizontal_space().into());

        let toolbar = Row::with_children(menu_items)
            .spacing(2)
            .padding(Padding::from([4, 8]))
            .align_y(iced::Alignment::Center);

        container(toolbar)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    /// Build a single dropdown menu item with label, shortcut, and action.
    fn menu_item<'a>(label: &'a str, shortcut: &'a str, msg: Message) -> Element<'a, Message> {
        button(
            row![
                text(label).size(12).color(colors::TEXT_PRIMARY),
                horizontal_space(),
                text(shortcut).size(11).color(colors::TEXT_MUTED),
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding::from([6, 16]))
        .style(|_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        })
        .on_press(msg)
        .into()
    }

    /// Build a disabled menu item (grayed out, no action).
    fn menu_separator<'a>() -> Element<'a, Message> {
        container(Space::new(Length::Fill, 1))
            .padding(Padding::from([4, 8]))
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into()
    }

    fn view_menu_dropdown(&self) -> Element<'_, Message> {
        let menu = match self.active_menu {
            Some(m) => m,
            None => return Space::new(0, 0).into(),
        };

        let mut items: Vec<Element<'_, Message>> = Vec::new();

        match menu {
            TopMenu::File => {
                items.push(Self::menu_item("New File", "Ctrl+N", Message::NewFile));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Open File...", "Ctrl+O", Message::OpenFile));
                items.push(Self::menu_item("Open Folder...", "", Message::OpenFolder));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Save", "Ctrl+S", Message::Save));
                items.push(Self::menu_item("Save As...", "Ctrl+Shift+S", Message::SaveAs));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Close Tab", "Ctrl+W", Message::CloseCurrentTab));
            }
            TopMenu::Edit => {
                items.push(Self::menu_item("Undo", "Ctrl+Z", Message::Undo));
                items.push(Self::menu_item("Redo", "Ctrl+Y", Message::Redo));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Cut", "Ctrl+X", Message::EditorCut));
                items.push(Self::menu_item("Copy", "Ctrl+C", Message::EditorCopy));
                items.push(Self::menu_item("Paste", "Ctrl+V", Message::EditorPaste));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Select All", "Ctrl+A", Message::EditorSelectAll));
            }
            TopMenu::Selection => {
                items.push(Self::menu_item("Select All", "Ctrl+A", Message::EditorSelectAll));
                items.push(Self::menu_item("Select Line", "", Message::SelectLine));
            }
            TopMenu::View => {
                items.push(Self::menu_item("Toggle Sidebar", "", Message::ToggleSidebar));
                items.push(Self::menu_item("Toggle Minimap", "", Message::ToggleMinimap));
                items.push(Self::menu_item("Toggle Terminal", "Ctrl+`", Message::ToggleTerminal));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Zoom In", "Ctrl+=", Message::ZoomIn));
                items.push(Self::menu_item("Zoom Out", "Ctrl+-", Message::ZoomOut));
                items.push(Self::menu_item("Reset Zoom", "Ctrl+0", Message::ZoomReset));
            }
            TopMenu::Go => {
                items.push(Self::menu_item("Go to Line...", "Ctrl+G", Message::ShowGotoLine));
                items.push(Self::menu_separator());
                items.push(Self::menu_item("Next Tab", "Ctrl+Tab", Message::NextTab));
                items.push(Self::menu_item("Previous Tab", "Ctrl+Shift+Tab", Message::PrevTab));
            }
            TopMenu::Window => {
                items.push(Self::menu_item("Close Window", "Ctrl+Q", Message::CloseWindow));
            }
            TopMenu::Help => {
                items.push(Self::menu_item("About Luminex", "", Message::ShowAbout));
            }
        }

        let menu_width = 240.0;
        let menu_content = Column::with_children(items)
            .width(Length::Fixed(menu_width))
            .padding(4);

        // Calculate horizontal offset based on which menu is open
        let menu_offset_x = match menu {
            TopMenu::File => 8.0,
            TopMenu::Edit => 52.0,
            TopMenu::Selection => 92.0,
            TopMenu::View => 160.0,
            TopMenu::Go => 200.0,
            TopMenu::Window => 228.0,
            TopMenu::Help => 286.0,
        };

        let menu_box = container(menu_content)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });

        // Position: toolbar height (~32px) + small gap
        column![
            Space::with_height(Length::Fixed(32.0)),
            row![
                Space::with_width(Length::Fixed(menu_offset_x)),
                menu_box,
            ],
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    // ========================================================================
    // Sidebar - File Tree
    // ========================================================================

    fn view_sidebar(&self) -> Element<'_, Message> {
        let small_btn_style = |_: &Theme, status: button::Status| -> button::Style {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                button::Status::Pressed => colors::BG_ACTIVE,
                _ => colors::BG_MEDIUM,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_SECONDARY,
                border: Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        };

        let header = container(
            row![
                text("EXPLORER")
                    .size(11)
                    .color(colors::TEXT_SECONDARY)
                    .font(Font::with_name("system-ui")),
                horizontal_space(),
                button(text("+F").size(10).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 6]))
                    .style(small_btn_style)
                    .on_press(Message::CreateNewFile),
                button(text("+D").size(10).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 6]))
                    .style(small_btn_style)
                    .on_press(Message::CreateFolder),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::from([10, 12]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(colors::BG_MEDIUM)),
            ..Default::default()
        });

        let file_content: Element<'_, Message> = if let Some(tree) = &self.file_tree {
            let items = self.build_file_tree_items(tree);
            // Wrap in mouse_area to detect right-clicks on empty space
            let tree_view = mouse_area(
                Column::with_children(items).spacing(1).width(Length::Fill)
            )
            .on_right_press(Message::ShowContextMenu(Point::ORIGIN, None, true));

            scrollable(tree_view)
                .height(Length::Fill)
                .into()
        } else {
            container(
                column![
                    Space::with_height(40),
                    text("No folder open").size(13).color(colors::TEXT_MUTED),
                    Space::with_height(16),
                    button(text("Open Folder").size(13).color(colors::ACCENT))
                        .padding(Padding::from([8, 16]))
                        .style(|_, status| {
                            let bg = match status {
                                button::Status::Hovered => colors::BG_HOVER,
                                _ => colors::BG_LIGHT,
                            };
                            button::Style {
                                background: Some(Background::Color(bg)),
                                text_color: colors::ACCENT,
                                border: Border {
                                    color: colors::ACCENT,
                                    width: 1.0,
                                    radius: 4.0.into(),
                                },
                                ..Default::default()
                            }
                        })
                        .on_press(Message::OpenFolder),
                ]
                .align_x(iced::Alignment::Center)
                .width(Length::Fill),
            )
            .height(Length::Fill)
            .into()
        };

        let sidebar_content = column![header, file_content];

        container(sidebar_content)
            .width(Length::Fixed(self.sidebar_width))
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_LIGHT)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn build_file_tree_items(&self, node: &FileNode) -> Vec<Element<'_, Message>> {
        let mut items = Vec::new();
        items.push(self.make_file_item(node));

        if node.expanded {
            for child in &node.children {
                items.extend(self.build_file_tree_items(child));
            }
        }

        items
    }

    fn make_file_item(&self, node: &FileNode) -> Element<'_, Message> {
        let icon = if node.is_dir {
            if node.expanded { "[-]" } else { "[+]" }
        } else {
            self.get_file_icon(&node.name)
        };

        let is_active = self
            .tabs
            .get(self.active_tab)
            .and_then(|t| t.path.as_ref())
            .map(|p| p == &node.path)
            .unwrap_or(false);

        let bg = if is_active {
            colors::BG_ACTIVE
        } else {
            Color::TRANSPARENT
        };

        let indent = (node.depth * 16 + 8) as f32;
        let path = node.path.clone();
        let path_for_menu = node.path.clone();
        let is_dir = node.is_dir;
        let name = node.name.clone();

        let item_btn = button(
            row![
                Space::with_width(Length::Fixed(indent)),
                text(icon).size(12).font(Font::MONOSPACE).color(colors::TEXT_MUTED),
                Space::with_width(6),
                text(name).size(13).color(if is_active {
                    colors::TEXT_PRIMARY
                } else {
                    colors::TEXT_SECONDARY
                }),
            ]
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .padding(Padding::from([4, 0]))
        .style(move |_, status| {
            let hover_bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => bg,
            };
            button::Style {
                background: Some(Background::Color(hover_bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        })
        .on_press(if is_dir {
            Message::ToggleFolder(path)
        } else {
            Message::FileClicked(path)
        });

        // Wrap in mouse_area for right-click support
        mouse_area(item_btn)
            .on_right_press(Message::ShowContextMenu(Point::ORIGIN, Some(path_for_menu), is_dir))
            .into()
    }

    fn get_file_icon(&self, name: &str) -> &'static str {
        let ext = name.rsplit('.').next().unwrap_or("");
        match ext {
            "rs" => " rs",
            "py" => " py",
            "js" | "ts" | "jsx" | "tsx" => " js",
            "html" => " <>",
            "css" | "scss" | "sass" => " cs",
            "json" | "toml" | "yaml" | "yml" => " {}",
            "md" => " md",
            "txt" => " tx",
            "sh" | "bash" | "zsh" => " sh",
            _ => "  .",
        }
    }

    // ========================================================================
    // Main Editor Area
    // ========================================================================

    fn view_main_area(&self) -> Element<'_, Message> {
        let mut editor_row_items: Vec<Element<'_, Message>> = vec![self.view_editor()];

        // Add minimap if visible
        if self.minimap_visible {
            editor_row_items.push(self.view_minimap());
        }

        // Add scrollbar indicator
        editor_row_items.push(self.view_scrollbar());

        let editor_with_minimap = Row::with_children(editor_row_items)
            .height(Length::Fill);

        let mut main_items: Vec<Element<'_, Message>> = vec![
            self.view_tabs(),
            editor_with_minimap.into(),
        ];

        // Add terminal if visible
        if self.terminal_visible {
            main_items.push(self.view_terminal_divider());
            main_items.push(self.view_terminal());
        }

        Column::with_children(main_items)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_tabs(&self) -> Element<'_, Message> {
        let mut tabs_vec: Vec<Element<'_, Message>> = Vec::new();

        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = self.active_tab == idx;
            tabs_vec.push(self.make_tab(&tab.name, idx, is_active, tab.modified));
        }

        tabs_vec.push(horizontal_space().into());

        let tabs_row = Row::with_children(tabs_vec)
            .spacing(1)
            .align_y(iced::Alignment::End);

        container(tabs_row)
            .width(Length::Fill)
            .height(36)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn make_tab(
        &self,
        name: &str,
        idx: usize,
        is_active: bool,
        modified: bool,
    ) -> Element<'_, Message> {
        let display_name = if modified {
            format!("{} *", name)
        } else {
            name.to_string()
        };

        let close_btn = button(text("x").size(12).color(colors::TEXT_MUTED))
            .padding(Padding::from([2, 6]))
            .style(|_, status| {
                let bg = match status {
                    button::Status::Hovered => colors::BG_HOVER,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: colors::TEXT_PRIMARY,
                    border: Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::CloseTab(idx));

        let tab_content = row![
            text(display_name).size(13).color(if is_active {
                colors::TEXT_PRIMARY
            } else {
                colors::TEXT_SECONDARY
            }),
            Space::with_width(8),
            close_btn,
        ]
        .align_y(iced::Alignment::Center);

        let bg = if is_active {
            colors::BG_DARK
        } else {
            colors::BG_MEDIUM
        };

        button(tab_content)
            .padding(Padding::from([8, 14]))
            .style(move |_, status| {
                let hover_bg = match status {
                    button::Status::Hovered if !is_active => colors::BG_HOVER,
                    _ => bg,
                };
                button::Style {
                    background: Some(Background::Color(hover_bg)),
                    text_color: colors::TEXT_PRIMARY,
                    border: Border {
                        color: if is_active {
                            colors::ACCENT
                        } else {
                            Color::TRANSPARENT
                        },
                        width: if is_active { 2.0 } else { 0.0 },
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }
            })
            .on_press(Message::TabSelected(idx))
            .into()
    }

    fn view_editor(&self) -> Element<'_, Message> {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let highlight_settings = HighlightSettings {
                language: tab.language.clone(),
            };

            // Selection color: bright blue normally, dimmed when context menu is open
            let context_open = self.editor_context_visible;
            let selection_color = if context_open {
                Color::from_rgba(0.30, 0.45, 0.75, 0.35)
            } else {
                Color::from_rgba(0.25, 0.46, 0.85, 0.55)
            };

            let editor = text_editor(&tab.content)
                .height(Length::Fill)
                .padding(16)
                .font(Font::MONOSPACE)
                .size(self.font_size)
                .style(move |_theme: &Theme, _status| {
                    text_editor::Style {
                        background: Background::Color(colors::BG_DARK),
                        border: Border {
                            width: 0.0,
                            radius: 0.0.into(),
                            color: Color::TRANSPARENT,
                        },
                        icon: colors::TEXT_MUTED,
                        placeholder: colors::TEXT_MUTED,
                        value: colors::TEXT_PRIMARY,
                        selection: selection_color,
                    }
                })
                .highlight_with::<EditorHighlighter>(highlight_settings, |highlight, _theme| {
                    highlight.to_format(Font::MONOSPACE)
                })
                .on_action(Message::EditorAction);

            let editor_container: Element<'_, Message> = container(editor)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

            // Wrap in mouse_area for right-click context menu
            mouse_area(editor_container)
                .on_right_press(Message::ShowEditorContextMenu)
                .into()
        } else {
            container(text("No file open").size(16).color(colors::TEXT_MUTED))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_DARK)),
                    ..Default::default()
                })
                .into()
        }
    }

    // ========================================================================
    // Scrollbar
    // ========================================================================

    fn view_scrollbar(&self) -> Element<'_, Message> {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let total_lines = tab.content.line_count().max(1);
            let (cursor_line, _) = tab.content.cursor_position();

            // Estimate visible lines based on font size (approximate)
            let visible_lines = (400.0 / (self.font_size * 1.4)) as usize;
            let visible_lines = visible_lines.max(10);

            // Calculate thumb size and position
            let thumb_ratio = (visible_lines as f32 / total_lines as f32).min(1.0);
            let scroll_ratio = if total_lines > visible_lines {
                cursor_line as f32 / (total_lines - 1).max(1) as f32
            } else {
                0.0
            };

            let thumb_height_pct = (thumb_ratio * 100.0).max(8.0).min(100.0);
            let thumb_top_pct = (scroll_ratio * (100.0 - thumb_height_pct)).max(0.0);

            // Build the scrollbar track with thumb
            let track: Element<'_, Message> = container(
                column![
                    Space::with_height(Length::FillPortion((thumb_top_pct * 100.0) as u16 + 1)),
                    container(Space::new(Length::Fill, Length::FillPortion((thumb_height_pct * 100.0) as u16 + 1)))
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.15))),
                            border: Border {
                                radius: 3.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    Space::with_height(Length::FillPortion(((100.0 - thumb_top_pct - thumb_height_pct) * 100.0) as u16 + 1)),
                ]
                .height(Length::Fill)
            )
            .width(Length::Fixed(12.0))
            .height(Length::Fill)
            .padding(Padding::from([2, 2]))
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                ..Default::default()
            })
            .into();

            track
        } else {
            // No file open, empty scrollbar area
            container(Space::new(12, Length::Fill))
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    ..Default::default()
                })
                .into()
        }
    }

    // ========================================================================
    // Minimap
    // ========================================================================

    fn view_minimap(&self) -> Element<'_, Message> {
        if let Some(tab) = self.tabs.get(self.active_tab) {
            let total_lines = tab.content.line_count();
            let (cursor_line, _) = tab.content.cursor_position();

            // Build minimap lines - show a compressed view of the code
            let max_minimap_lines = 200;
            let lines_to_show = total_lines.min(max_minimap_lines);

            let mut minimap_items: Vec<Element<'_, Message>> = Vec::new();

            for i in 0..lines_to_show {
                // Sample lines if there are too many
                let line_idx = if total_lines <= max_minimap_lines {
                    i
                } else {
                    (i as f64 * total_lines as f64 / max_minimap_lines as f64) as usize
                };

                let line_text = tab.content.line(line_idx)
                    .map(|l| {
                        let s: &str = &l;
                        s.to_string()
                    })
                    .unwrap_or_default();

                // Create a visual representation of the line
                let line_len = line_text.trim_end().len().min(80);
                let indent = line_text.len() - line_text.trim_start().len();
                let indent = indent.min(40);
                let content_len = if line_len > indent { line_len - indent } else { 0 };

                let is_current = line_idx == cursor_line;
                let alpha = if is_current { 0.8 } else { 0.25 };
                let line_color = if is_current {
                    Color::from_rgba(0.36, 0.54, 0.90, 0.6)
                } else {
                    Color::from_rgba(0.7, 0.7, 0.7, alpha)
                };

                let minimap_line: Element<'_, Message> = row![
                    Space::with_width(Length::Fixed(indent as f32 * 0.8)),
                    container(Space::new(Length::Fixed((content_len as f32 * 0.8).max(1.0)), 2))
                        .style(move |_| container::Style {
                            background: Some(Background::Color(line_color)),
                            ..Default::default()
                        }),
                ]
                .into();

                minimap_items.push(minimap_line);
            }

            // Viewport indicator
            let minimap_content = Column::with_children(minimap_items)
                .spacing(0)
                .width(Length::Fill);

            container(
                scrollable(minimap_content)
                    .height(Length::Fill)
            )
            .width(Length::Fixed(80.0))
            .height(Length::Fill)
            .padding(Padding::from([4, 4]))
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.14))),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
        } else {
            Space::new(0, 0).into()
        }
    }

    // ========================================================================
    // Terminal
    // ========================================================================

    fn view_terminal_divider(&self) -> Element<'_, Message> {
        container(Space::new(Length::Fill, 1))
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into()
    }

    fn view_terminal(&self) -> Element<'_, Message> {
        // Terminal header
        let header = container(
            row![
                text("TERMINAL").size(11).color(colors::TEXT_SECONDARY).font(Font::MONOSPACE),
                horizontal_space(),
                text(self.terminal_cwd.display().to_string())
                    .size(10)
                    .color(colors::TEXT_MUTED)
                    .font(Font::MONOSPACE),
                Space::with_width(8),
                button(text("Clear").size(10).color(colors::TEXT_SECONDARY).font(Font::MONOSPACE))
                    .padding(Padding::from([2, 8]))
                    .style(|_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => colors::BG_HOVER,
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: colors::TEXT_SECONDARY,
                            border: Border {
                                radius: 3.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    })
                    .on_press(Message::TerminalClear),
                Space::with_width(4),
                button(text("x").size(11).color(colors::TEXT_MUTED))
                    .padding(Padding::from([2, 6]))
                    .style(|_: &Theme, status: button::Status| {
                        let bg = match status {
                            button::Status::Hovered => colors::BG_HOVER,
                            _ => Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: colors::TEXT_PRIMARY,
                            border: Border {
                                radius: 2.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    })
                    .on_press(Message::ToggleTerminal),
            ]
            .spacing(4)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::from([6, 12]))
        .width(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgb(0.10, 0.10, 0.12))),
            ..Default::default()
        });

        // Terminal output
        let mut output_items: Vec<Element<'_, Message>> = Vec::new();
        for line in &self.terminal_output {
            let line_color = if line.starts_with("$ ") {
                colors::ACCENT
            } else if line.starts_with("Error:") || line.starts_with("error") {
                Color::from_rgb(0.90, 0.35, 0.35)
            } else {
                colors::TEXT_SECONDARY
            };
            output_items.push(
                text(line).size(12).font(Font::MONOSPACE).color(line_color).into()
            );
        }

        let output_column = Column::with_children(output_items)
            .spacing(1)
            .width(Length::Fill);

        let output_scroll = scrollable(output_column)
            .height(Length::Fill);

        // Terminal input line
        let prompt_display: String = format!("{}$", self.terminal_cwd
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string()));

        let input_line = row![
            text(prompt_display).size(12).font(Font::MONOSPACE).color(colors::ACCENT),
            Space::with_width(6),
            text_input("Type a command...", &self.terminal_input)
                .on_input(Message::TerminalInputChanged)
                .on_submit(Message::TerminalSubmit)
                .padding(Padding::from([4, 8]))
                .size(12)
                .font(Font::MONOSPACE),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(0);

        let terminal_content = column![
            header,
            container(output_scroll)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding::from([4, 12])),
            container(input_line)
                .width(Length::Fill)
                .padding(Padding::from([6, 12]))
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.09, 0.09, 0.11))),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..Default::default()
                }),
        ];

        container(terminal_content)
            .width(Length::Fill)
            .height(Length::Fixed(self.terminal_height))
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgb(0.08, 0.08, 0.10))),
                ..Default::default()
            })
            .into()
    }

    fn editor_menu_btn<'a>(label: &'a str, shortcut: &'a str, msg: Message, enabled: bool) -> Element<'a, Message> {
        let text_color = if enabled { colors::TEXT_PRIMARY } else { colors::TEXT_MUTED };
        let shortcut_color = colors::TEXT_MUTED;

        let btn = button(
            row![
                text(label).size(12).color(text_color),
                horizontal_space(),
                text(shortcut).size(11).color(shortcut_color),
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding(Padding::from([6, 12]))
        .style(|_: &Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                _ => colors::BG_MEDIUM,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border::default(),
                ..Default::default()
            }
        });

        if enabled {
            btn.on_press(msg).into()
        } else {
            btn.into()
        }
    }

    fn editor_menu_separator<'a>() -> Element<'a, Message> {
        container(Space::new(Length::Fill, 1))
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BORDER)),
                ..Default::default()
            })
            .into()
    }

    fn view_editor_context_menu(&self) -> Element<'_, Message> {
        let has_selection = self.tabs.get(self.active_tab)
            .and_then(|t| t.content.selection())
            .is_some();

        let mut items: Vec<Element<'_, Message>> = Vec::new();

        // Undo / Redo
        items.push(Self::editor_menu_btn("Undo", "Ctrl+Z", Message::Undo, true));
        items.push(Self::editor_menu_btn("Redo", "Ctrl+Y", Message::Redo, true));
        items.push(Self::editor_menu_separator());

        // Cut / Copy / Paste
        items.push(Self::editor_menu_btn("Cut", "Ctrl+X", Message::EditorCut, has_selection));
        items.push(Self::editor_menu_btn("Copy", "Ctrl+C", Message::EditorCopy, has_selection));
        items.push(Self::editor_menu_btn("Paste", "Ctrl+V", Message::EditorPaste, true));
        items.push(Self::editor_menu_separator());

        // Select All
        items.push(Self::editor_menu_btn("Select All", "Ctrl+A", Message::EditorSelectAll, true));

        let menu_content = Column::with_children(items).width(Length::Fixed(220.0));

        let x = self.editor_context_position.x;
        let y = self.editor_context_position.y;

        let menu_box = container(menu_content)
            .padding(4)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        column![
            Space::with_height(Length::Fixed(y)),
            row![
                Space::with_width(Length::Fixed(x)),
                menu_box,
            ],
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    // ========================================================================
    // Status Bar
    // ========================================================================

    fn view_status_bar(&self) -> Element<'_, Message> {
        let cursor_info = if let Some(tab) = self.tabs.get(self.active_tab) {
            let (line, col) = tab.content.cursor_position();
            format!("Ln {}, Col {}", line + 1, col + 1)
        } else {
            "Ln 1, Col 1".to_string()
        };

        let file_info = self
            .tabs
            .get(self.active_tab)
            .map(|t| {
                if t.modified {
                    format!("{} [modified]", t.name)
                } else {
                    t.name.clone()
                }
            })
            .unwrap_or_else(|| "No file".to_string());

        let language_info = self
            .tabs
            .get(self.active_tab)
            .map(|t| t.language.clone())
            .unwrap_or_else(|| "text".to_string());

        let status_content = row![
            text(&self.status_message)
                .size(12)
                .color(colors::TEXT_SECONDARY),
            horizontal_space(),
            text(file_info).size(12).color(colors::TEXT_SECONDARY),
            Space::with_width(24),
            text(cursor_info).size(12).color(colors::TEXT_PRIMARY),
            Space::with_width(24),
            text(language_info).size(12).color(colors::ACCENT),
            Space::with_width(24),
            text("UTF-8").size(12).color(colors::TEXT_SECONDARY),
            Space::with_width(12),
        ]
        .padding(Padding::from([6, 12]))
        .align_y(iced::Alignment::Center);

        container(status_content)
            .width(Length::Fill)
            .height(28)
            .style(|_| container::Style {
                background: Some(Background::Color(colors::BG_MEDIUM)),
                border: Border {
                    color: colors::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

// ============================================================================
// Run Application
// ============================================================================

pub fn run(_flags: Flags) -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .window_size(iced::Size::new(1280.0, 800.0))
        .theme(|_| Theme::Dark)
        .antialiasing(true)
        .run_with(App::new)
}
