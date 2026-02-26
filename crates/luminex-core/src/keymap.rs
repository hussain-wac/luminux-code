//! Keyboard mapping and input handling.
//!
//! ## Learning: State Machines
//!
//! Key handling often involves state machines:
//! - Normal state: Process keys immediately
//! - Pending state: Waiting for more keys (e.g., `gg`)
//! - Chord state: Modifier held (e.g., Ctrl+K, Ctrl+C)
//!
//! This enables complex keybindings like Vim and VS Code.

use crate::command::Command;
use crate::config::Config;
use crate::editor::EditorMode;
use std::collections::HashMap;

/// Keyboard modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool, // Cmd on macOS, Win on Windows
}

impl Modifiers {
    /// No modifiers pressed.
    pub const NONE: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: false,
        meta: false,
    };

    /// Ctrl modifier.
    pub const CTRL: Modifiers = Modifiers {
        ctrl: true,
        alt: false,
        shift: false,
        meta: false,
    };

    /// Shift modifier.
    pub const SHIFT: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: true,
        meta: false,
    };

    /// Alt modifier.
    pub const ALT: Modifiers = Modifiers {
        ctrl: false,
        alt: true,
        shift: false,
        meta: false,
    };

    /// Meta (Cmd/Win) modifier.
    pub const META: Modifiers = Modifiers {
        ctrl: false,
        alt: false,
        shift: false,
        meta: true,
    };

    /// Ctrl+Shift.
    pub const CTRL_SHIFT: Modifiers = Modifiers {
        ctrl: true,
        alt: false,
        shift: true,
        meta: false,
    };

    /// Returns true if no modifiers are pressed.
    pub fn is_empty(&self) -> bool {
        !self.ctrl && !self.alt && !self.shift && !self.meta
    }

    /// Parses modifiers from a string like "ctrl+shift".
    pub fn parse(s: &str) -> Self {
        let mut mods = Modifiers::NONE;
        let lower = s.to_lowercase();
        if lower.contains("ctrl") || lower.contains("control") {
            mods.ctrl = true;
        }
        if lower.contains("alt") || lower.contains("option") {
            mods.alt = true;
        }
        if lower.contains("shift") {
            mods.shift = true;
        }
        if lower.contains("meta") || lower.contains("cmd") || lower.contains("win") {
            mods.meta = true;
        }
        mods
    }
}

impl std::fmt::Display for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.meta {
            #[cfg(target_os = "macos")]
            parts.push("Cmd");
            #[cfg(not(target_os = "macos"))]
            parts.push("Win");
        }
        write!(f, "{}", parts.join("+"))
    }
}

/// A key code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    F(u8), // F1-F12
    Space,
}

impl Key {
    /// Parses a key from a string.
    pub fn parse(s: &str) -> Option<Self> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "enter" | "return" => Some(Key::Enter),
            "tab" => Some(Key::Tab),
            "backspace" | "bs" => Some(Key::Backspace),
            "delete" | "del" => Some(Key::Delete),
            "escape" | "esc" => Some(Key::Escape),
            "up" => Some(Key::Up),
            "down" => Some(Key::Down),
            "left" => Some(Key::Left),
            "right" => Some(Key::Right),
            "home" => Some(Key::Home),
            "end" => Some(Key::End),
            "pageup" | "pgup" => Some(Key::PageUp),
            "pagedown" | "pgdn" => Some(Key::PageDown),
            "insert" | "ins" => Some(Key::Insert),
            "space" => Some(Key::Space),
            _ if lower.starts_with('f') && lower.len() <= 3 => {
                lower[1..].parse().ok().map(Key::F)
            }
            _ if s.chars().count() == 1 => Some(Key::Char(s.chars().next().unwrap())),
            _ => None,
        }
    }
}

impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::Char(c) => write!(f, "{}", c.to_uppercase()),
            Key::Enter => write!(f, "Enter"),
            Key::Tab => write!(f, "Tab"),
            Key::Backspace => write!(f, "Backspace"),
            Key::Delete => write!(f, "Delete"),
            Key::Escape => write!(f, "Escape"),
            Key::Up => write!(f, "Up"),
            Key::Down => write!(f, "Down"),
            Key::Left => write!(f, "Left"),
            Key::Right => write!(f, "Right"),
            Key::Home => write!(f, "Home"),
            Key::End => write!(f, "End"),
            Key::PageUp => write!(f, "PageUp"),
            Key::PageDown => write!(f, "PageDown"),
            Key::Insert => write!(f, "Insert"),
            Key::F(n) => write!(f, "F{}", n),
            Key::Space => write!(f, "Space"),
        }
    }
}

/// A key press event.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyPress {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl KeyPress {
    /// Creates a new key press.
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Parses a key binding string like "ctrl+s" or "g g".
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        let key_str = parts.last()?;
        let key = Key::parse(key_str)?;

        let mod_str = parts[..parts.len() - 1].join("+");
        let modifiers = Modifiers::parse(&mod_str);

        Some(Self { key, modifiers })
    }
}

impl std::fmt::Display for KeyPress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers.is_empty() {
            write!(f, "{}", self.key)
        } else {
            write!(f, "{}+{}", self.modifiers, self.key)
        }
    }
}

/// A key binding maps a key sequence to a command.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// The key sequence (may be multiple keys for chords).
    pub keys: Vec<KeyPress>,
    /// The command to execute.
    pub command: Command,
    /// Mode(s) in which this binding is active.
    pub modes: Vec<EditorMode>,
    /// When clause (future: conditions for activation).
    pub when: Option<String>,
}

impl KeyBinding {
    /// Creates a simple key binding.
    pub fn simple(key: KeyPress, command: Command) -> Self {
        Self {
            keys: vec![key],
            command,
            modes: vec![
                EditorMode::Normal,
                EditorMode::Insert,
                EditorMode::Visual,
                EditorMode::Command,
            ],
            when: None,
        }
    }

    /// Creates a mode-specific binding.
    pub fn for_mode(key: KeyPress, command: Command, mode: EditorMode) -> Self {
        Self {
            keys: vec![key],
            command,
            modes: vec![mode],
            when: None,
        }
    }

    /// Returns the key sequence as a string.
    pub fn key_string(&self) -> String {
        self.keys
            .iter()
            .map(|k| k.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Keyboard mapping configuration.
pub struct Keymap {
    /// All key bindings.
    bindings: Vec<KeyBinding>,
    /// Index by first key for fast lookup.
    by_key: HashMap<KeyPress, Vec<usize>>,
    /// Current pending keys (for multi-key sequences).
    pending: Vec<KeyPress>,
}

impl Keymap {
    /// Creates a new keymap with default bindings.
    pub fn new() -> Self {
        let mut keymap = Self {
            bindings: Vec::new(),
            by_key: HashMap::new(),
            pending: Vec::new(),
        };
        keymap.add_default_bindings();
        keymap.rebuild_index();
        keymap
    }

    /// Creates a keymap from configuration.
    pub fn from_config(config: &Config) -> Self {
        let mut keymap = Self::new();

        // Add user bindings
        for (key_str, cmd_str) in &config.keyboard.bindings {
            if let Some(key) = KeyPress::parse(key_str) {
                if let Some(cmd) = Self::parse_command(cmd_str) {
                    keymap.bindings.push(KeyBinding::simple(key, cmd));
                }
            }
        }

        keymap.rebuild_index();
        keymap
    }

    /// Adds default key bindings.
    fn add_default_bindings(&mut self) {
        use crate::command::Command::*;
        use EditorMode::{Insert, Normal, Visual};

        let bindings = vec![
            // File operations
            (
                KeyPress::new(Key::Char('n'), Modifiers::CTRL),
                NewFile,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('o'), Modifiers::CTRL),
                OpenFile { path: None },
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('s'), Modifiers::CTRL),
                Save,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('w'), Modifiers::CTRL),
                CloseFile,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('q'), Modifiers::CTRL),
                Quit,
                vec![Normal, Insert],
            ),
            // Edit operations
            (
                KeyPress::new(Key::Char('z'), Modifiers::CTRL),
                Undo,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('y'), Modifiers::CTRL),
                Redo,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('x'), Modifiers::CTRL),
                Cut,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('c'), Modifiers::CTRL),
                Copy,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('v'), Modifiers::CTRL),
                Paste,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('a'), Modifiers::CTRL),
                SelectAll,
                vec![Normal, Insert],
            ),
            // Cursor movement (Insert mode)
            (
                KeyPress::new(Key::Up, Modifiers::NONE),
                MoveUp { count: 1 },
                vec![Insert],
            ),
            (
                KeyPress::new(Key::Down, Modifiers::NONE),
                MoveDown { count: 1 },
                vec![Insert],
            ),
            (
                KeyPress::new(Key::Left, Modifiers::NONE),
                MoveLeft { count: 1 },
                vec![Insert],
            ),
            (
                KeyPress::new(Key::Right, Modifiers::NONE),
                MoveRight { count: 1 },
                vec![Insert],
            ),
            (
                KeyPress::new(Key::Home, Modifiers::NONE),
                MoveToLineStart,
                vec![Insert],
            ),
            (
                KeyPress::new(Key::End, Modifiers::NONE),
                MoveToLineEnd,
                vec![Insert],
            ),
            // Vim-style normal mode (if enabled)
            (
                KeyPress::new(Key::Char('h'), Modifiers::NONE),
                MoveLeft { count: 1 },
                vec![Normal],
            ),
            (
                KeyPress::new(Key::Char('j'), Modifiers::NONE),
                MoveDown { count: 1 },
                vec![Normal],
            ),
            (
                KeyPress::new(Key::Char('k'), Modifiers::NONE),
                MoveUp { count: 1 },
                vec![Normal],
            ),
            (
                KeyPress::new(Key::Char('l'), Modifiers::NONE),
                MoveRight { count: 1 },
                vec![Normal],
            ),
            (
                KeyPress::new(Key::Char('i'), Modifiers::NONE),
                EnterInsertMode,
                vec![Normal],
            ),
            (
                KeyPress::new(Key::Escape, Modifiers::NONE),
                EnterNormalMode,
                vec![Insert, Visual],
            ),
            // View
            (
                KeyPress::new(Key::Char('='), Modifiers::CTRL),
                ZoomIn,
                vec![Normal, Insert],
            ),
            (
                KeyPress::new(Key::Char('-'), Modifiers::CTRL),
                ZoomOut,
                vec![Normal, Insert],
            ),
        ];

        for (key, cmd, modes) in bindings {
            self.bindings.push(KeyBinding {
                keys: vec![key],
                command: cmd,
                modes,
                when: None,
            });
        }
    }

    /// Rebuilds the key index.
    fn rebuild_index(&mut self) {
        self.by_key.clear();
        for (i, binding) in self.bindings.iter().enumerate() {
            if let Some(first_key) = binding.keys.first() {
                self.by_key
                    .entry(first_key.clone())
                    .or_insert_with(Vec::new)
                    .push(i);
            }
        }
    }

    /// Parses a command string.
    fn parse_command(s: &str) -> Option<Command> {
        // Simple command parsing - expand as needed
        match s {
            "editor.save" => Some(Command::Save),
            "editor.undo" => Some(Command::Undo),
            "editor.redo" => Some(Command::Redo),
            "editor.quit" => Some(Command::Quit),
            _ => None,
        }
    }

    /// Processes a key press.
    ///
    /// Returns Some(Command) if a binding matches, None if waiting for more keys.
    pub fn process(&mut self, key: KeyPress, mode: EditorMode) -> KeymapResult {
        self.pending.push(key.clone());

        // Find matching bindings
        let first_key = &self.pending[0];
        let indices = match self.by_key.get(first_key) {
            Some(v) => v.clone(),
            None => {
                self.pending.clear();
                return KeymapResult::NoMatch;
            }
        };

        let mut exact_match = None;
        let mut prefix_match = false;

        for i in indices {
            let binding = &self.bindings[i];

            // Check mode
            if !binding.modes.contains(&mode) {
                continue;
            }

            // Check if this binding matches our pending keys
            if binding.keys.len() == self.pending.len() {
                if binding.keys == self.pending {
                    exact_match = Some(binding.command.clone());
                }
            } else if binding.keys.len() > self.pending.len() {
                // Could be a prefix
                if binding.keys[..self.pending.len()] == self.pending[..] {
                    prefix_match = true;
                }
            }
        }

        if let Some(cmd) = exact_match {
            self.pending.clear();
            return KeymapResult::Match(cmd);
        }

        if prefix_match {
            return KeymapResult::Pending;
        }

        self.pending.clear();
        KeymapResult::NoMatch
    }

    /// Clears pending keys.
    pub fn clear_pending(&mut self) {
        self.pending.clear();
    }

    /// Returns true if waiting for more keys.
    pub fn is_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Returns all bindings.
    pub fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    /// Adds a binding.
    pub fn add_binding(&mut self, binding: KeyBinding) {
        self.bindings.push(binding);
        self.rebuild_index();
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of processing a key.
#[derive(Debug, Clone)]
pub enum KeymapResult {
    /// A command was matched.
    Match(Command),
    /// Waiting for more keys.
    Pending,
    /// No binding matches.
    NoMatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypress_parse() {
        let kp = KeyPress::parse("ctrl+s").unwrap();
        assert_eq!(kp.key, Key::Char('s'));
        assert!(kp.modifiers.ctrl);
    }

    #[test]
    fn test_keymap_match() {
        let mut keymap = Keymap::new();
        let result = keymap.process(
            KeyPress::new(Key::Char('s'), Modifiers::CTRL),
            EditorMode::Insert,
        );
        assert!(matches!(result, KeymapResult::Match(Command::Save)));
    }
}
