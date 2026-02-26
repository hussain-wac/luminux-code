use std::path::{Path, PathBuf};
use iced::Point;
use iced::widget::text_editor;
use crate::highlighter::detect_language;

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
    pub fn from_path(path: &Path, depth: u16) -> Option<Self> {
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

    pub fn load_children(&mut self) {
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

pub struct TabInfo {
    pub path: Option<PathBuf>,
    pub name: String,
    pub content: text_editor::Content,
    pub modified: bool,
    pub language: String,
    // Undo/redo history
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
    #[allow(dead_code)]
    pub last_saved_content: String,
}

impl TabInfo {
    pub fn new_untitled(id: usize) -> Self {
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

    pub fn from_file(path: PathBuf, text: String) -> Self {
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

    pub fn save_undo_state(&mut self) {
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

    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            let current = self.content.text();
            self.redo_stack.push(current);
            self.content = text_editor::Content::with_text(&previous);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
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

#[derive(Debug, Default)]
pub struct Flags {
    pub file: Option<String>,
    pub workspace: Option<String>,
}

/// Context menu state
#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub visible: bool,
    pub position: Point,
    pub target: Option<PathBuf>,
    #[allow(dead_code)]
    pub is_directory: bool,
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
pub struct DeletedEntry {
    /// Original path of the deleted item.
    pub path: PathBuf,
    /// If the item was a file, its contents. `None` for directories.
    pub content: Option<Vec<u8>>,
    /// If the item was a directory, a recursive snapshot of its contents.
    pub children: Vec<DeletedEntry>,
    pub is_dir: bool,
}

impl DeletedEntry {
    /// Recursively snapshot a path before deleting it.
    pub fn snapshot(path: &Path) -> Option<Self> {
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
    pub fn restore(&self) -> Result<(), String> {
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
