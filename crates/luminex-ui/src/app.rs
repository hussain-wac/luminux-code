//! Main application state and logic.
//!
//! A fully functional text editor with file browsing and editing.

use iced::keyboard;
use iced::widget::{
    button, column, container, horizontal_space, row, scrollable, stack, text, text_editor, Column, Row,
    Space, mouse_area,
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
                self.untitled_counter += 1;
                self.tabs.push(TabInfo::new_untitled(self.untitled_counter));
                self.active_tab = self.tabs.len() - 1;
                self.status_message = "New file created".to_string();
            }

            Message::OpenFile => {
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
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if tab.undo() {
                        self.status_message = "Undo".to_string();
                    } else {
                        self.status_message = "Nothing to undo".to_string();
                    }
                }
            }

            Message::Redo => {
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    if tab.redo() {
                        self.status_message = "Redo".to_string();
                    } else {
                        self.status_message = "Nothing to redo".to_string();
                    }
                }
            }

            Message::EditorAction(action) => {
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
                if idx < self.tabs.len() {
                    self.active_tab = idx;
                    let name = self.tabs[idx].name.clone();
                    self.status_message = format!("Editing: {}", name);
                }
            }

            Message::NextTab => {
                if !self.tabs.is_empty() {
                    self.active_tab = (self.active_tab + 1) % self.tabs.len();
                    let name = self.tabs[self.active_tab].name.clone();
                    self.status_message = format!("Switched to: {}", name);
                }
            }

            Message::PrevTab => {
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
                if let Some(tree) = &mut self.file_tree {
                    Self::toggle_folder_recursive(tree, &path);
                }
            }

            Message::ToggleSidebar => {
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
                    self.status_message = format!("Deleted: {}", name);
                }
                Err(e) => {
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

            // Context menu messages
            Message::ShowContextMenu(position, target, is_directory) => {
                self.context_menu = ContextMenu {
                    visible: true,
                    position,
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
                // Note: Full rename requires a text input dialog, which is complex in iced
                // For now, show a message
                self.status_message = "Rename: Use your file manager for now".to_string();
            }

        }
        Task::none()
    }

    fn refresh_file_tree(&mut self) {
        if let Some(current_folder) = &self.current_folder {
            if let Some(mut tree) = FileNode::from_path(current_folder, 0) {
                tree.expanded = true;
                tree.load_children();
                self.file_tree = Some(tree);
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

        // If context menu is visible, show it as overlay
        if self.context_menu.visible {
            stack![
                // Click-away layer to close menu
                mouse_area(
                    container(Space::new(Length::Fill, Length::Fill))
                        .width(Length::Fill)
                        .height(Length::Fill)
                )
                .on_press(Message::HideContextMenu),
                main_view,
                self.view_context_menu(),
            ]
            .into()
        } else {
            main_view
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

        // Position the menu (fixed position since we can't get exact cursor position in iced)
        // We position it at top-left of sidebar area as a simple approach
        container(
            container(menu_content)
                .padding(4)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_MEDIUM)),
                    border: Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
        )
        .padding(Padding::from([50, 12]))
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
    }

    // ========================================================================
    // Toolbar
    // ========================================================================

    fn view_toolbar(&self) -> Element<'_, Message> {
        let btn_style = |_: &Theme, status: button::Status| -> button::Style {
            let bg = match status {
                button::Status::Hovered => colors::BG_HOVER,
                button::Status::Pressed => colors::BG_ACTIVE,
                _ => colors::BG_LIGHT,
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: colors::TEXT_PRIMARY,
                border: Border {
                    radius: 4.0.into(),
                    color: colors::BORDER,
                    width: 1.0,
                },
                ..Default::default()
            }
        };

        let sidebar_btn = button(text("[=]").size(12).font(Font::MONOSPACE))
            .padding(Padding::from([6, 10]))
            .style(btn_style)
            .on_press(Message::ToggleSidebar);

        let new_btn = button(text("New").size(12))
            .padding(Padding::from([6, 12]))
            .style(btn_style)
            .on_press(Message::NewFile);

        let open_file_btn = button(text("Open").size(12))
            .padding(Padding::from([6, 12]))
            .style(btn_style)
            .on_press(Message::OpenFile);

        let open_folder_btn = button(text("Folder").size(12))
            .padding(Padding::from([6, 12]))
            .style(btn_style)
            .on_press(Message::OpenFolder);

        let save_btn = button(text("Save").size(12))
            .padding(Padding::from([6, 12]))
            .style(btn_style)
            .on_press(Message::Save);

        let toolbar = row![
            sidebar_btn,
            Space::with_width(12),
            new_btn,
            open_file_btn,
            open_folder_btn,
            save_btn,
            horizontal_space(),
            text("Ctrl+Z: Undo | Ctrl+Y: Redo | Ctrl+C/V/X: Copy/Paste/Cut")
                .size(11)
                .color(colors::TEXT_MUTED),
            Space::with_width(12),
        ]
        .spacing(4)
        .padding(Padding::from([8, 12]))
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
        column![self.view_tabs(), self.view_editor(),]
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

            let editor = text_editor(&tab.content)
                .height(Length::Fill)
                .padding(16)
                .font(Font::MONOSPACE)
                .size(14)
                .highlight_with::<EditorHighlighter>(highlight_settings, |highlight, _theme| {
                    highlight.to_format(Font::MONOSPACE)
                })
                .on_action(Message::EditorAction);

            container(editor)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(colors::BG_DARK)),
                    ..Default::default()
                })
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
