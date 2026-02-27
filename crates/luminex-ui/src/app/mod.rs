use iced::{Point, Task, Subscription, keyboard};
use iced::widget::text_editor;
use std::path::{Path, PathBuf};

pub mod types;
pub mod messages;
pub mod update;
pub mod file_ops;
pub mod view;

pub use types::*;
pub use messages::*;

pub struct App {
    pub tabs: Vec<TabInfo>,
    pub active_tab: usize,
    pub sidebar_visible: bool,
    pub sidebar_width: f32,
    pub file_tree: Option<FileNode>,
    pub current_folder: Option<PathBuf>,
    pub status_message: String,
    pub untitled_counter: usize,
    pub context_menu: ContextMenu,
    pub clipboard_path: Option<PathBuf>,
    pub clipboard_is_cut: bool,
    pub last_cursor_position: Point,
    pub deleted_stack: Vec<DeletedEntry>,
    pub confirm_delete_visible: bool,
    pub confirm_delete_target: Option<PathBuf>,
    pub minimap_visible: bool,
    pub rename_visible: bool,
    pub rename_target: Option<PathBuf>,
    pub rename_input: String,
    pub editor_context_visible: bool,
    pub editor_context_position: Point,
    pub active_menu: Option<TopMenu>,
    pub font_size: f32,
    pub goto_line_visible: bool,
    pub goto_line_input: String,
    pub about_visible: bool,
    pub right_dock_visible: bool,
    pub outline_visible: bool,
    pub diagnostics_visible: bool,
    pub diagnostics_messages: Vec<String>,
    pub terminal_visible: bool,
    pub terminal_height: f32,
    pub is_resizing_terminal: bool,
    pub terminal_focused: bool,
    pub terminal: iced_term::Terminal,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
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
            right_dock_visible: true,
            outline_visible: false,
            diagnostics_visible: false,
            diagnostics_messages: Vec::new(),
            terminal_visible: false,
            terminal_height: 250.0,
            is_resizing_terminal: false,
            terminal_focused: false,
            terminal: iced_term::Terminal::new(0, iced_term::settings::Settings {
                backend: iced_term::settings::BackendSettings {
                    program: if cfg!(windows) { "powershell.exe".to_string() } else { std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string()) },
                    args: vec![],
                    ..Default::default()
                },
                ..Default::default()
            }).expect("failed to create the new terminal instance"),
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

"#;
        if let Some(tab) = app.tabs.get_mut(0) {
            tab.content = text_editor::Content::with_text(welcome_text);
            tab.language = "rust".to_string();
        }

        (app, Task::none())
    }

    pub fn title(&self) -> String {
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

    pub fn subscription(&self) -> Subscription<Message> {
        let keyboard_sub = keyboard::on_key_press(|key, modifiers| {
            Some(Message::KeyPressed(key, modifiers))
        });

        Subscription::batch([
            keyboard_sub,
            iced::Subscription::run_with_id(
                self.terminal.id,
                self.terminal.subscription(),
            ).map(Message::TerminalEvent),
        ])
    }
}

pub fn run(_flags: Flags) -> iced::Result {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .window_size(iced::Size::new(1280.0, 800.0))
        .theme(|_| iced::Theme::Dark)
        .antialiasing(true)
        .run_with(App::new)
}
