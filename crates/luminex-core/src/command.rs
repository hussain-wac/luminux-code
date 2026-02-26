//! Command system for editor actions.
//!
//! ## Learning: The Command Pattern
//!
//! Commands encapsulate actions as objects:
//! - Actions become first-class values
//! - Can be stored, queued, serialized
//! - Enables undo/redo, macros, key bindings
//!
//! ## Trait Objects vs Enums
//!
//! We use an enum for built-in commands (exhaustive, no allocation)
//! and trait objects for plugin commands (extensible, heap allocated).

use crate::editor::Editor;
use crate::CoreResult;
use std::collections::HashMap;

/// Built-in editor commands.
///
/// ## Learning: Exhaustive Enums
///
/// With `#[non_exhaustive]`, we signal that new variants may be added.
/// Match arms should include `_ =>` to handle future variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Command {
    // File commands
    NewFile,
    OpenFile { path: Option<String> },
    Save,
    SaveAs { path: Option<String> },
    CloseFile,
    CloseAll,
    Quit,

    // Edit commands
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Delete,
    DeleteLine,
    DuplicateLine,

    // Cursor movement
    MoveUp { count: usize },
    MoveDown { count: usize },
    MoveLeft { count: usize },
    MoveRight { count: usize },
    MoveToLineStart,
    MoveToLineEnd,
    MoveToFileStart,
    MoveToFileEnd,
    MoveWordLeft,
    MoveWordRight,
    PageUp,
    PageDown,

    // Selection
    SelectUp { count: usize },
    SelectDown { count: usize },
    SelectLeft { count: usize },
    SelectRight { count: usize },
    SelectLine,
    SelectWord,

    // Search
    Find,
    FindNext,
    FindPrevious,
    Replace,
    GotoLine,

    // View
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleSidebar,
    ToggleTerminal,
    SplitVertical,
    SplitHorizontal,

    // Mode
    EnterInsertMode,
    EnterNormalMode,
    EnterVisualMode,
    EnterCommandMode,

    // Custom command (name, arguments)
    Custom { name: String, args: Vec<String> },
}

impl Command {
    /// Returns the command's display name.
    pub fn display_name(&self) -> &str {
        match self {
            Command::NewFile => "New File",
            Command::OpenFile { .. } => "Open File",
            Command::Save => "Save",
            Command::SaveAs { .. } => "Save As",
            Command::CloseFile => "Close File",
            Command::CloseAll => "Close All",
            Command::Quit => "Quit",
            Command::Undo => "Undo",
            Command::Redo => "Redo",
            Command::Cut => "Cut",
            Command::Copy => "Copy",
            Command::Paste => "Paste",
            Command::SelectAll => "Select All",
            Command::Delete => "Delete",
            Command::DeleteLine => "Delete Line",
            Command::DuplicateLine => "Duplicate Line",
            Command::MoveUp { .. } => "Move Up",
            Command::MoveDown { .. } => "Move Down",
            Command::MoveLeft { .. } => "Move Left",
            Command::MoveRight { .. } => "Move Right",
            Command::MoveToLineStart => "Move to Line Start",
            Command::MoveToLineEnd => "Move to Line End",
            Command::MoveToFileStart => "Move to File Start",
            Command::MoveToFileEnd => "Move to File End",
            Command::MoveWordLeft => "Move Word Left",
            Command::MoveWordRight => "Move Word Right",
            Command::PageUp => "Page Up",
            Command::PageDown => "Page Down",
            Command::SelectUp { .. } => "Select Up",
            Command::SelectDown { .. } => "Select Down",
            Command::SelectLeft { .. } => "Select Left",
            Command::SelectRight { .. } => "Select Right",
            Command::SelectLine => "Select Line",
            Command::SelectWord => "Select Word",
            Command::Find => "Find",
            Command::FindNext => "Find Next",
            Command::FindPrevious => "Find Previous",
            Command::Replace => "Replace",
            Command::GotoLine => "Go to Line",
            Command::ZoomIn => "Zoom In",
            Command::ZoomOut => "Zoom Out",
            Command::ZoomReset => "Zoom Reset",
            Command::ToggleSidebar => "Toggle Sidebar",
            Command::ToggleTerminal => "Toggle Terminal",
            Command::SplitVertical => "Split Vertical",
            Command::SplitHorizontal => "Split Horizontal",
            Command::EnterInsertMode => "Enter Insert Mode",
            Command::EnterNormalMode => "Enter Normal Mode",
            Command::EnterVisualMode => "Enter Visual Mode",
            Command::EnterCommandMode => "Enter Command Mode",
            Command::Custom { name, .. } => name,
        }
    }
}

/// Context passed to command execution.
pub struct CommandContext<'a> {
    pub editor: &'a mut Editor,
}

/// Trait for custom command handlers.
///
/// ## Learning: Trait Objects
///
/// `dyn CommandHandler` allows storing different types that
/// implement this trait in the same collection. The `Send + Sync`
/// bounds ensure thread safety.
pub trait CommandHandler: Send + Sync {
    /// Returns the command name.
    fn name(&self) -> &str;

    /// Executes the command.
    fn execute(&self, ctx: &mut CommandContext, args: &[String]) -> CoreResult<()>;

    /// Returns a description for the command palette.
    fn description(&self) -> &str {
        self.name()
    }
}

/// Registry for commands.
///
/// ## Learning: Type Erasure
///
/// `Box<dyn CommandHandler>` erases the concrete type, allowing
/// different handler types in the same HashMap. The vtable (virtual
/// table) enables dynamic dispatch.
pub struct CommandRegistry {
    /// Custom command handlers
    handlers: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    /// Creates a new registry.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers a custom command handler.
    pub fn register(&mut self, handler: Box<dyn CommandHandler>) {
        let name = handler.name().to_string();
        self.handlers.insert(name, handler);
    }

    /// Executes a command.
    pub fn execute(&self, cmd: &Command, editor: &mut Editor) -> CoreResult<()> {
        let mut ctx = CommandContext { editor };

        match cmd {
            // File commands
            Command::NewFile => {
                ctx.editor.new_document();
                Ok(())
            }
            Command::OpenFile { path } => {
                if let Some(p) = path {
                    ctx.editor.open_file(p)?;
                }
                // If no path, UI should show file picker
                Ok(())
            }
            Command::Save => ctx.editor.save_current(),
            Command::SaveAs { path } => {
                if let Some(p) = path {
                    ctx.editor.save_current_as(p)
                } else {
                    Ok(())
                }
            }
            Command::Quit => {
                ctx.editor.quit();
                Ok(())
            }

            // Edit commands
            Command::Undo => ctx.editor.undo(),
            Command::Redo => ctx.editor.redo(),
            Command::Cut => ctx.editor.cut(),
            Command::Copy => ctx.editor.copy(),
            Command::Paste => ctx.editor.paste(),
            Command::SelectAll => ctx.editor.select_all(),

            // Movement commands
            Command::MoveUp { count } => ctx.editor.move_up(*count),
            Command::MoveDown { count } => ctx.editor.move_down(*count),
            Command::MoveLeft { count } => ctx.editor.move_left(*count),
            Command::MoveRight { count } => ctx.editor.move_right(*count),
            Command::MoveToLineStart => ctx.editor.move_to_line_start(),
            Command::MoveToLineEnd => ctx.editor.move_to_line_end(),

            // Mode commands
            Command::EnterInsertMode => {
                ctx.editor.enter_insert_mode();
                Ok(())
            }
            Command::EnterNormalMode => {
                ctx.editor.enter_normal_mode();
                Ok(())
            }

            // Custom commands
            Command::Custom { name, args } => {
                if let Some(handler) = self.handlers.get(name) {
                    handler.execute(&mut ctx, args)
                } else {
                    Err(crate::CoreError::CommandNotFound(name.clone()))
                }
            }

            // Default: not implemented yet
            _ => {
                tracing::debug!("Command not implemented: {:?}", cmd);
                Ok(())
            }
        }
    }

    /// Returns all registered command names.
    pub fn list(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_display_name() {
        assert_eq!(Command::Save.display_name(), "Save");
        assert_eq!(
            Command::Custom {
                name: "my_cmd".to_string(),
                args: vec![]
            }
            .display_name(),
            "my_cmd"
        );
    }
}
