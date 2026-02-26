//! # Luminex Plugin System
//!
//! Extensible plugin architecture for the editor.
//!
//! ## Plugin Types
//!
//! 1. **Native plugins**: Compiled Rust code loaded as dynamic libraries
//! 2. **Script plugins**: Lua/WASM scripts (future)
//!
//! ## Learning: Dynamic Loading
//!
//! Native plugins use `libloading` to load shared libraries (.so/.dll/.dylib)
//! at runtime. This allows extending the editor without recompilation.
//!
//! ## Safety Considerations
//!
//! Loading native code is inherently unsafe:
//! - Plugins must be trusted
//! - ABI compatibility must be maintained
//! - Memory safety depends on plugin implementation

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Plugin system errors.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Failed to load plugin: {0}")]
    LoadFailed(String),

    #[error("Plugin already loaded: {0}")]
    AlreadyLoaded(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Incompatible plugin version: expected {expected}, got {got}")]
    IncompatibleVersion { expected: String, got: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Plugin API version for compatibility checking.
pub const API_VERSION: &str = "0.1.0";

/// Plugin manifest (plugin.toml).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Version string
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Author information
    pub author: Option<String>,

    /// Repository URL
    pub repository: Option<String>,

    /// License
    pub license: Option<String>,

    /// Required API version
    pub api_version: String,

    /// Plugin entry point (library name or script path)
    pub main: String,

    /// Plugin type
    #[serde(default)]
    pub plugin_type: PluginType,

    /// Dependencies on other plugins
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Activation events
    #[serde(default)]
    pub activation_events: Vec<String>,

    /// Commands contributed by this plugin
    #[serde(default)]
    pub commands: Vec<CommandContribution>,

    /// Keybindings contributed by this plugin
    #[serde(default)]
    pub keybindings: Vec<KeybindingContribution>,

    /// Languages contributed by this plugin
    #[serde(default)]
    pub languages: Vec<LanguageContribution>,
}

/// Plugin type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Native Rust plugin (dynamic library)
    #[default]
    Native,
    /// Lua script plugin
    Lua,
    /// WebAssembly plugin
    Wasm,
}

/// Command contribution from a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    /// Command identifier
    pub id: String,
    /// Display name
    pub title: String,
    /// Category (for command palette grouping)
    #[serde(default)]
    pub category: Option<String>,
}

/// Keybinding contribution from a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingContribution {
    /// Command to execute
    pub command: String,
    /// Key sequence
    pub key: String,
    /// When clause
    #[serde(default)]
    pub when: Option<String>,
}

/// Language contribution from a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageContribution {
    /// Language identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// File extensions
    pub extensions: Vec<String>,
    /// File name patterns
    #[serde(default)]
    pub filenames: Vec<String>,
}

/// Plugin state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin is installed but not loaded
    Installed,
    /// Plugin is being loaded
    Loading,
    /// Plugin is loaded and active
    Active,
    /// Plugin encountered an error
    Error,
    /// Plugin is disabled
    Disabled,
}

/// Information about a loaded plugin.
#[derive(Debug)]
pub struct PluginInfo {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Plugin directory
    pub path: PathBuf,
    /// Current state
    pub state: PluginState,
    /// Error message (if state is Error)
    pub error: Option<String>,
}

/// The Plugin trait that native plugins must implement.
///
/// ## Learning: Trait Objects for Plugins
///
/// By defining a trait, we can:
/// - Load different plugin implementations uniformly
/// - Use dynamic dispatch for plugin methods
/// - Define a stable ABI boundary
pub trait Plugin: Send + Sync {
    /// Returns the plugin name.
    fn name(&self) -> &str;

    /// Called when the plugin is activated.
    fn activate(&mut self, ctx: &mut PluginContext) -> Result<(), PluginError>;

    /// Called when the plugin is deactivated.
    fn deactivate(&mut self) -> Result<(), PluginError>;

    /// Called when a command is executed.
    fn execute_command(
        &mut self,
        command: &str,
        args: &[String],
        ctx: &mut PluginContext,
    ) -> Result<(), PluginError>;
}

/// Context passed to plugins for interacting with the editor.
pub struct PluginContext {
    // API methods for plugins to interact with the editor
    // This would be expanded with actual editor API calls
}

impl PluginContext {
    pub fn new() -> Self {
        Self {}
    }

    /// Logs a message.
    pub fn log(&self, level: &str, message: &str) {
        match level {
            "error" => tracing::error!(plugin = true, "{}", message),
            "warn" => tracing::warn!(plugin = true, "{}", message),
            "info" => tracing::info!(plugin = true, "{}", message),
            "debug" => tracing::debug!(plugin = true, "{}", message),
            _ => tracing::trace!(plugin = true, "{}", message),
        }
    }

    /// Shows a notification to the user.
    pub fn show_notification(&self, message: &str) {
        tracing::info!(notification = true, "{}", message);
        // Would integrate with UI notification system
    }

    /// Registers a command handler.
    pub fn register_command(&mut self, _id: &str, _handler: Box<dyn Fn(&[String]) + Send + Sync>) {
        // Would register with command system
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin manager.
pub struct PluginManager {
    /// Plugin directory
    plugins_dir: PathBuf,

    /// Loaded plugins
    plugins: HashMap<String, PluginInfo>,

    /// Plugin context
    context: PluginContext,
}

impl PluginManager {
    /// Creates a new plugin manager.
    pub fn new(plugins_dir: impl Into<PathBuf>) -> Self {
        Self {
            plugins_dir: plugins_dir.into(),
            plugins: HashMap::new(),
            context: PluginContext::new(),
        }
    }

    /// Discovers installed plugins.
    pub fn discover(&mut self) -> Result<Vec<String>, PluginError> {
        let mut discovered = Vec::new();

        if !self.plugins_dir.exists() {
            std::fs::create_dir_all(&self.plugins_dir)?;
            return Ok(discovered);
        }

        for entry in std::fs::read_dir(&self.plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let manifest_path = path.join("plugin.toml");
                if manifest_path.exists() {
                    match self.load_manifest(&manifest_path) {
                        Ok(manifest) => {
                            let id = manifest.id.clone();
                            self.plugins.insert(
                                id.clone(),
                                PluginInfo {
                                    manifest,
                                    path,
                                    state: PluginState::Installed,
                                    error: None,
                                },
                            );
                            discovered.push(id);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load plugin manifest: {}", e);
                        }
                    }
                }
            }
        }

        Ok(discovered)
    }

    /// Loads a plugin manifest.
    fn load_manifest(&self, path: &Path) -> Result<PluginManifest, PluginError> {
        let content = std::fs::read_to_string(path)?;
        let manifest: PluginManifest =
            toml::from_str(&content).map_err(|e| PluginError::InvalidManifest(e.to_string()))?;

        // Version compatibility check
        if !Self::is_compatible(&manifest.api_version) {
            return Err(PluginError::IncompatibleVersion {
                expected: API_VERSION.to_string(),
                got: manifest.api_version,
            });
        }

        Ok(manifest)
    }

    /// Checks if an API version is compatible.
    fn is_compatible(version: &str) -> bool {
        // Simple major version check
        let current_major = API_VERSION.split('.').next().unwrap_or("0");
        let plugin_major = version.split('.').next().unwrap_or("0");
        current_major == plugin_major
    }

    /// Activates a plugin.
    pub fn activate(&mut self, id: &str) -> Result<(), PluginError> {
        let info = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;

        if info.state == PluginState::Active {
            return Ok(());
        }

        info.state = PluginState::Loading;

        // In a real implementation, we would:
        // 1. Load the native library or script
        // 2. Create the plugin instance
        // 3. Call activate()

        info.state = PluginState::Active;
        tracing::info!("Activated plugin: {}", id);

        Ok(())
    }

    /// Deactivates a plugin.
    pub fn deactivate(&mut self, id: &str) -> Result<(), PluginError> {
        let info = self
            .plugins
            .get_mut(id)
            .ok_or_else(|| PluginError::NotFound(id.to_string()))?;

        if info.state != PluginState::Active {
            return Ok(());
        }

        // Would call deactivate() on the plugin instance

        info.state = PluginState::Installed;
        tracing::info!("Deactivated plugin: {}", id);

        Ok(())
    }

    /// Returns information about a plugin.
    pub fn get(&self, id: &str) -> Option<&PluginInfo> {
        self.plugins.get(id)
    }

    /// Returns all plugins.
    pub fn list(&self) -> impl Iterator<Item = &PluginInfo> {
        self.plugins.values()
    }

    /// Returns active plugins.
    pub fn active(&self) -> impl Iterator<Item = &PluginInfo> {
        self.plugins
            .values()
            .filter(|p| p.state == PluginState::Active)
    }

    /// Returns all contributed commands.
    pub fn commands(&self) -> Vec<&CommandContribution> {
        self.active()
            .flat_map(|p| p.manifest.commands.iter())
            .collect()
    }

    /// Returns all contributed keybindings.
    pub fn keybindings(&self) -> Vec<&KeybindingContribution> {
        self.active()
            .flat_map(|p| p.manifest.keybindings.iter())
            .collect()
    }

    /// Returns all contributed languages.
    pub fn languages(&self) -> Vec<&LanguageContribution> {
        self.active()
            .flat_map(|p| p.manifest.languages.iter())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_parsing() {
        let manifest_toml = r#"
            id = "test-plugin"
            name = "Test Plugin"
            version = "1.0.0"
            description = "A test plugin"
            api_version = "0.1.0"
            main = "test_plugin"

            [[commands]]
            id = "test.hello"
            title = "Hello World"
        "#;

        let manifest: PluginManifest = toml::from_str(manifest_toml).unwrap();
        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.commands.len(), 1);
    }

    #[test]
    fn test_plugin_manager() {
        let dir = tempdir().unwrap();
        let mut manager = PluginManager::new(dir.path());

        // Should work with empty directory
        let discovered = manager.discover().unwrap();
        assert!(discovered.is_empty());
    }
}
