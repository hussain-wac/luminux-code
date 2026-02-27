use iced::{Task, keyboard};
use iced::widget::text_editor;

use super::{App, Message, TabInfo, FileNode, DeletedEntry, TopMenu};
use crate::highlighter::detect_language;

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
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

                    while final_path.exists() {
                        final_path = current_folder.join(format!("New Folder {}", counter));
                        counter += 1;
                    }

                    if let Err(e) = std::fs::create_dir(&final_path) {
                        self.status_message = format!("Failed to create folder: {}", e);
                    } else {
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
                if self.editor_context_visible {
                    match &action {
                        text_editor::Action::Click(_) | text_editor::Action::Drag(_) => {
                            self.editor_context_visible = false;
                            self.context_menu.visible = false;
                            self.active_menu = None;
                            return Task::none();
                        }
                        _ => {
                            self.editor_context_visible = false;
                        }
                    }
                }

                self.context_menu.visible = false;
                self.active_menu = None;
                self.terminal_focused = false;

                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    let is_edit = action.is_edit();
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
                        return self.update(Message::FileClicked(final_path));
                    }
                } else {
                    self.status_message = "Open a folder first".to_string();
                }
            }

            Message::MouseMoved(p) => {
                if self.is_resizing_terminal {
                    let delta_y = self.last_cursor_position.y - p.y;
                    self.terminal_height = (self.terminal_height + delta_y).clamp(50.0, 800.0);
                }
                self.last_cursor_position = p;
            }

            Message::StartTerminalResize => {
                self.is_resizing_terminal = true;
            }

            Message::StopTerminalResize => {
                self.is_resizing_terminal = false;
            }

            Message::ShowContextMenu(_position, target, is_directory) => {
                self.context_menu = crate::app::ContextMenu {
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
                    if let Some(snapshot) = DeletedEntry::snapshot(&target) {
                        self.deleted_stack.push(snapshot);
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
                        }
                    }
                } else {
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

            Message::ShowEditorContextMenu => {
                self.editor_context_visible = true;
                self.editor_context_position = self.last_cursor_position;
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
            Message::ZoomResetAll => {
                self.active_menu = None;
                self.font_size = 14.0;
                self.status_message = "All zoom reset to 14px".to_string();
            }

            Message::ToggleLeftDock => {
                self.active_menu = None;
                self.sidebar_visible = !self.sidebar_visible;
                self.status_message = if self.sidebar_visible {
                    "Left dock shown".to_string()
                } else {
                    "Left dock hidden".to_string()
                };
            }
            Message::ToggleRightDock => {
                self.active_menu = None;
                self.right_dock_visible = !self.right_dock_visible;
                self.status_message = if self.right_dock_visible {
                    "Right dock shown".to_string()
                } else {
                    "Right dock hidden".to_string()
                };
            }
            Message::ToggleBottomDock => {
                self.active_menu = None;
                self.terminal_visible = !self.terminal_visible;
                self.diagnostics_visible = false;
                if self.terminal_visible {
                    self.terminal_focused = true;
                    self.status_message = "Bottom dock shown".to_string();
                } else {
                    self.terminal_focused = false;
                    self.status_message = "Bottom dock hidden".to_string();
                }
            }
            Message::ToggleAllDocks => {
                self.active_menu = None;
                let any_visible = self.sidebar_visible || self.right_dock_visible || self.terminal_visible || self.diagnostics_visible;
                if any_visible {
                    self.sidebar_visible = false;
                    self.right_dock_visible = false;
                    self.terminal_visible = false;
                    self.diagnostics_visible = false;
                    self.status_message = "All docks hidden".to_string();
                } else {
                    self.sidebar_visible = true;
                    self.right_dock_visible = true;
                    self.status_message = "All docks shown".to_string();
                }
            }

            Message::ToggleProjectPanel => {
                self.active_menu = None;
                self.sidebar_visible = !self.sidebar_visible;
                self.status_message = if self.sidebar_visible {
                    "Project panel shown".to_string()
                } else {
                    "Project panel hidden".to_string()
                };
            }
            Message::ToggleOutlinePanel => {
                self.active_menu = None;
                self.outline_visible = !self.outline_visible;
                if self.outline_visible {
                    self.right_dock_visible = true;
                }
                self.status_message = if self.outline_visible {
                    "Outline panel shown".to_string()
                } else {
                    "Outline panel hidden".to_string()
                };
            }
            Message::ToggleTerminalPanel => {
                self.active_menu = None;
                self.terminal_visible = !self.terminal_visible;
                if self.terminal_visible {
                    self.diagnostics_visible = false;
                    self.status_message = "Terminal panel shown".to_string();
                } else {
                    self.status_message = "Terminal panel hidden".to_string();
                }
            }
            Message::ToggleDiagnostics => {
                self.active_menu = None;
                self.diagnostics_visible = !self.diagnostics_visible;
                if self.diagnostics_visible {
                    self.terminal_visible = false;
                }
                self.status_message = if self.diagnostics_visible {
                    "Diagnostics panel shown".to_string()
                } else {
                    "Diagnostics panel hidden".to_string()
                };
            }

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

            Message::SelectLine => {
                self.active_menu = None;
                self.editor_context_visible = false;
                if let Some(tab) = self.tabs.get_mut(self.active_tab) {
                    tab.content.perform(text_editor::Action::SelectLine);
                    self.status_message = "Line selected".to_string();
                }
            }

            Message::CloseWindow => {
                self.active_menu = None;
                return iced::exit();
            }

            Message::ShowAbout => {
                self.active_menu = None;
                self.about_visible = true;
            }
            Message::HideAbout => {
                self.about_visible = false;
            }

            Message::TerminalEvent(event) => {
                match event {
                    iced_term::Event::BackendCall(_, cmd) => {
                        match self.terminal.handle(iced_term::Command::ProxyToBackend(cmd)) {
                            iced_term::actions::Action::Shutdown => {}
                            _ => {}
                        }
                    }
                }
            }
            Message::TerminalClear => {}
            Message::TerminalFocused => {
                self.terminal_focused = true;
            }

            Message::KeyPressed(key, modifiers) => {
                return self.handle_key_pressed(key, modifiers);
            }
        }
        Task::none()
    }

    pub fn handle_key_pressed(&mut self, key: keyboard::Key, modifiers: keyboard::Modifiers) -> Task<Message> {
        if modifiers.control() {
            let char_key = match &key {
                keyboard::Key::Character(c) => Some(c.to_lowercase()),
                _ => None,
            };

            if let Some(c) = char_key {
                if modifiers.shift() {
                    match c.as_str() {
                        "s" => return self.update(Message::SaveAs),
                        "z" => return self.update(Message::Redo),
                        "e" => return self.update(Message::ToggleProjectPanel),
                        "b" => return self.update(Message::ToggleOutlinePanel),
                        "m" => return self.update(Message::ToggleDiagnostics),
                        _ => {}
                    }
                }

                if modifiers.alt() {
                    match c.as_str() {
                        "b" => return self.update(Message::ToggleRightDock),
                        "y" => return self.update(Message::ToggleAllDocks),
                        _ => {}
                    }
                }

                if !modifiers.shift() && !modifiers.alt() {
                    match c.as_str() {
                        "`" => return self.update(Message::ToggleTerminalPanel),
                        "j" => return self.update(Message::ToggleBottomDock),
                        "q" => return self.update(Message::CloseWindow),
                        _ => {}
                    }
                    match c.as_str() {
                        "a" => return self.update(Message::EditorSelectAll),
                        "n" => return self.update(Message::NewFile),
                        "o" => return self.update(Message::OpenFile),
                        "s" => return self.update(Message::Save),
                        "w" => return self.update(Message::CloseCurrentTab),
                        "z" => return self.update(Message::Undo),
                        "y" => return self.update(Message::Redo),
                        "g" => return self.update(Message::ShowGotoLine),
                        "=" | "+" => return self.update(Message::ZoomIn),
                        "-" => return self.update(Message::ZoomOut),
                        "0" => return self.update(Message::ZoomReset),
                        "b" => return self.update(Message::ToggleLeftDock),
                        _ => {}
                    }
                }
            }

            if matches!(key, keyboard::Key::Named(keyboard::key::Named::Tab)) {
                if modifiers.shift() {
                    return self.update(Message::PrevTab);
                } else {
                    return self.update(Message::NextTab);
                }
            }

            return Task::none();
        }

        Task::none()
    }
}
