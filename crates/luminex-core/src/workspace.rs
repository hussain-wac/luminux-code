//! Workspace management for project-based editing.
//!
//! ## Learning: Async File Operations
//!
//! File I/O is inherently slow (disk/network access).
//! Using async allows the UI to remain responsive while
//! files are being read/written in the background.

use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};

use crate::{CoreError, CoreResult};

/// Represents a workspace (project folder).
pub struct Workspace {
    /// Root directory of the workspace
    root: PathBuf,

    /// Workspace name
    name: String,

    /// File tree structure
    tree: FileTree,

    /// File watcher for detecting external changes
    #[allow(dead_code)]
    watcher: Option<RecommendedWatcher>,

    /// Channel for file change events
    #[allow(dead_code)]
    change_rx: Option<mpsc::Receiver<FileChange>>,
}

impl Workspace {
    /// Opens a workspace from a directory.
    pub fn open(path: impl AsRef<Path>) -> CoreResult<Self> {
        let root = path.as_ref().canonicalize()?;

        if !root.is_dir() {
            return Err(CoreError::FileNotFound(root.display().to_string()));
        }

        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Workspace")
            .to_string();

        // Build initial file tree
        let tree = FileTree::from_path(&root)?;

        Ok(Self {
            root,
            name,
            tree,
            watcher: None,
            change_rx: None,
        })
    }

    /// Starts watching for file changes.
    pub fn start_watching(&mut self) -> CoreResult<mpsc::Receiver<FileChange>> {
        let (tx, rx) = mpsc::channel(100);
        let root = self.root.clone();

        // Create watcher
        let tx_clone = tx.clone();
        let watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let change = match event.kind {
                        notify::EventKind::Create(_) => FileChange::Created(path),
                        notify::EventKind::Modify(_) => FileChange::Modified(path),
                        notify::EventKind::Remove(_) => FileChange::Deleted(path),
                        _ => continue,
                    };
                    let _ = tx_clone.blocking_send(change);
                }
            }
        })?;

        let mut watcher = watcher;
        watcher.watch(&root, RecursiveMode::Recursive)?;

        self.watcher = Some(watcher);

        Ok(rx)
    }

    /// Returns the workspace root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the workspace name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the file tree.
    pub fn tree(&self) -> &FileTree {
        &self.tree
    }

    /// Refreshes the file tree.
    pub fn refresh(&mut self) -> CoreResult<()> {
        self.tree = FileTree::from_path(&self.root)?;
        Ok(())
    }

    /// Resolves a path relative to the workspace root.
    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    /// Returns the relative path from the workspace root.
    pub fn relative(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        path.as_ref().strip_prefix(&self.root).ok().map(PathBuf::from)
    }

    /// Checks if a path is within the workspace.
    pub fn contains(&self, path: impl AsRef<Path>) -> bool {
        path.as_ref().starts_with(&self.root)
    }

    /// Finds files matching a glob pattern.
    pub fn find_files(&self, pattern: &str) -> Vec<PathBuf> {
        self.tree.find_files(pattern, &self.root)
    }
}

/// File change notification.
#[derive(Debug, Clone)]
pub enum FileChange {
    Created(PathBuf),
    Modified(PathBuf),
    Deleted(PathBuf),
}

/// A tree structure representing files and directories.
#[derive(Debug, Clone)]
pub struct FileTree {
    /// Root node
    pub root: FileNode,
}

impl FileTree {
    /// Creates a file tree from a directory path.
    pub fn from_path(path: &Path) -> CoreResult<Self> {
        let root = Self::build_node(path, 3)?; // Max 3 levels deep initially
        Ok(Self { root })
    }

    /// Builds a file node recursively.
    fn build_node(path: &Path, depth: usize) -> CoreResult<FileNode> {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if path.is_file() {
            return Ok(FileNode {
                name,
                path: path.to_path_buf(),
                kind: NodeKind::File,
                children: Vec::new(),
                expanded: false,
            });
        }

        let mut children = Vec::new();

        if depth > 0 {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    let entry_name = entry.file_name().to_string_lossy().to_string();

                    // Skip hidden files and common ignore patterns
                    if entry_name.starts_with('.')
                        || entry_name == "node_modules"
                        || entry_name == "target"
                        || entry_name == "__pycache__"
                    {
                        continue;
                    }

                    if let Ok(child) = Self::build_node(&entry_path, depth - 1) {
                        children.push(child);
                    }
                }

                // Sort: directories first, then alphabetically
                children.sort_by(|a, b| {
                    match (&a.kind, &b.kind) {
                        (NodeKind::Directory, NodeKind::File) => std::cmp::Ordering::Less,
                        (NodeKind::File, NodeKind::Directory) => std::cmp::Ordering::Greater,
                        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    }
                });
            }
        }

        Ok(FileNode {
            name,
            path: path.to_path_buf(),
            kind: NodeKind::Directory,
            children,
            expanded: depth > 0,
        })
    }

    /// Finds files matching a simple pattern.
    pub fn find_files(&self, pattern: &str, root: &Path) -> Vec<PathBuf> {
        let mut results = Vec::new();
        self.find_files_recursive(&self.root, pattern, root, &mut results);
        results
    }

    fn find_files_recursive(
        &self,
        node: &FileNode,
        pattern: &str,
        root: &Path,
        results: &mut Vec<PathBuf>,
    ) {
        // Simple pattern matching (contains)
        if node.name.contains(pattern) {
            results.push(node.path.clone());
        }

        for child in &node.children {
            self.find_files_recursive(child, pattern, root, results);
        }
    }

    /// Expands a directory node to load its children.
    pub fn expand(&mut self, path: &Path) -> CoreResult<()> {
        Self::expand_node(&mut self.root, path)
    }
}

impl FileTree {
    /// Recursively expand a node (static function to avoid borrow issues).
    fn expand_node(node: &mut FileNode, target: &Path) -> CoreResult<()> {
        if node.path == target {
            if node.kind == NodeKind::Directory && node.children.is_empty() {
                *node = Self::build_node(&node.path, 1)?;
                node.expanded = true;
            }
            return Ok(());
        }

        for child in &mut node.children {
            if target.starts_with(&child.path) {
                Self::expand_node(child, target)?;
            }
        }

        Ok(())
    }
}

/// A node in the file tree.
#[derive(Debug, Clone)]
pub struct FileNode {
    /// File/directory name
    pub name: String,

    /// Full path
    pub path: PathBuf,

    /// Node type
    pub kind: NodeKind,

    /// Child nodes (for directories)
    pub children: Vec<FileNode>,

    /// Whether the node is expanded (for directories)
    pub expanded: bool,
}

impl FileNode {
    /// Returns the file extension, if any.
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    /// Returns true if this is a directory.
    pub fn is_directory(&self) -> bool {
        self.kind == NodeKind::Directory
    }

    /// Returns true if this is a file.
    pub fn is_file(&self) -> bool {
        self.kind == NodeKind::File
    }

    /// Returns the icon name for this file type.
    pub fn icon(&self) -> &'static str {
        match self.kind {
            NodeKind::Directory => "folder",
            NodeKind::File => match self.extension() {
                Some("rs") => "rust",
                Some("py") => "python",
                Some("js" | "jsx") => "javascript",
                Some("ts" | "tsx") => "typescript",
                Some("html") => "html",
                Some("css" | "scss" | "sass") => "css",
                Some("json") => "json",
                Some("toml") => "toml",
                Some("yaml" | "yml") => "yaml",
                Some("md") => "markdown",
                Some("git" | "gitignore") => "git",
                _ => "file",
            },
        }
    }
}

/// Type of file tree node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    File,
    Directory,
}

impl From<notify::Error> for CoreError {
    fn from(err: notify::Error) -> Self {
        CoreError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            err.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_workspace_open() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let ws = Workspace::open(dir.path()).unwrap();
        assert!(ws.contains(dir.path().join("test.txt")));
    }

    #[test]
    fn test_file_tree() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main() {}").unwrap();

        let tree = FileTree::from_path(dir.path()).unwrap();
        assert_eq!(tree.root.kind, NodeKind::Directory);
    }
}
