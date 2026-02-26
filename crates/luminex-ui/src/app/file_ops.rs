use std::path::{Path, PathBuf};

use super::{App, FileNode};

impl App {
    pub fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
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

    pub fn refresh_file_tree(&mut self) {
        if let Some(current_folder) = &self.current_folder {
            let mut expanded_paths = std::collections::HashSet::new();
            if let Some(old_tree) = &self.file_tree {
                Self::collect_expanded_paths(old_tree, &mut expanded_paths);
            }

            if let Some(mut tree) = FileNode::from_path(current_folder, 0) {
                tree.expanded = true;
                tree.load_children();
                Self::restore_expanded_state(&mut tree, &expanded_paths);
                self.file_tree = Some(tree);
            }
        }
    }

    pub fn collect_expanded_paths(node: &FileNode, set: &mut std::collections::HashSet<PathBuf>) {
        if node.expanded {
            set.insert(node.path.clone());
            for child in &node.children {
                Self::collect_expanded_paths(child, set);
            }
        }
    }

    pub fn restore_expanded_state(node: &mut FileNode, expanded: &std::collections::HashSet<PathBuf>) {
        if node.is_dir && expanded.contains(&node.path) {
            node.expanded = true;
            node.load_children();
            for child in &mut node.children {
                Self::restore_expanded_state(child, expanded);
            }
        }
    }

    pub fn toggle_folder_recursive(node: &mut FileNode, target: &Path) {
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
}
