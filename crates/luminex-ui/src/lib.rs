//! # Luminex UI
//!
//! Modern, beautiful UI using the iced framework.
//!
//! ## Architecture
//!
//! The UI follows the Elm architecture (TEA):
//! - **Model**: Application state
//! - **Message**: Events that can occur
//! - **Update**: Pure function: (state, message) -> new state
//! - **View**: Pure function: state -> UI elements
//!
//! ## Learning: The Elm Architecture
//!
//! This architecture provides:
//! - Predictable state updates
//! - Easy debugging (state is just data)
//! - Testable logic (pure functions)
//! - Time-travel debugging (replay messages)

pub mod app;
pub mod components;
pub mod highlighter;
pub mod style;
pub mod theme;

pub use app::{run, App, Flags};
pub use theme::Theme;
