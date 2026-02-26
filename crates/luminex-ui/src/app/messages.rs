use iced::{Point, keyboard};
use iced::widget::text_editor;
use std::path::PathBuf;

use crate::app::types::TopMenu;

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
    ZoomResetAll,

    // Dock toggles
    ToggleLeftDock,
    ToggleRightDock,
    ToggleBottomDock,
    ToggleAllDocks,

    // Panels
    ToggleProjectPanel,
    ToggleOutlinePanel,
    ToggleTerminalPanel,
    ToggleDiagnostics,

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
    TerminalEvent(iced_term::Event),
    TerminalClear,
    TerminalFocused,
    KeyPressed(keyboard::Key, keyboard::Modifiers),

    // Async results
    FileOpened(Result<(PathBuf, String), String>),
    FolderOpened(Result<PathBuf, String>),
    FileSaved(Result<PathBuf, String>),
    FileDeleted(Result<PathBuf, String>),
}
