# Luminex

A modern, fast, and beautiful text editor built in Rust.

## Features

- **High Performance**: Built with Rust for memory safety and speed
- **Modern UI**: Clean, minimal interface with dark/light themes
- **Syntax Highlighting**: Tree-sitter based incremental parsing
- **LSP Support**: Language Server Protocol for intelligent code completion
- **Plugin System**: Extensible architecture for custom functionality
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Architecture

```
luminex/
├── src/                    # Main application entry point
├── crates/
│   ├── luminex-core/       # Core editor logic and state management
│   ├── luminex-buffer/     # Text buffer with rope data structure
│   ├── luminex-ui/         # UI components using iced framework
│   ├── luminex-syntax/     # Syntax highlighting with tree-sitter
│   ├── luminex-lsp/        # Language Server Protocol client
│   └── luminex-plugin/     # Plugin system
├── assets/
│   ├── themes/             # Color themes
│   ├── icons/              # UI icons
│   └── fonts/              # Editor fonts
├── docs/                   # Documentation
├── benches/                # Performance benchmarks
└── tests/                  # Integration tests
```

## Quick Start

### Prerequisites

- Rust 1.75+ (latest stable recommended)
- Platform-specific dependencies:
  - **Linux**: `libxcb`, `libxkbcommon`
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools

### Build & Run

```bash
# Clone the repository
git clone https://github.com/yourusername/luminex.git
cd luminex

# Build in release mode
cargo build --release

# Run the editor
cargo run --release

# Run with a file
cargo run --release -- path/to/file.rs

# Run with a workspace
cargo run --release -- --workspace path/to/project
```

## Development Roadmap

### Phase 1: MVP ✅
- [x] Project structure and architecture
- [x] Text buffer with rope data structure
- [x] Cursor and selection management
- [x] Undo/redo history
- [x] Basic UI framework
- [x] File operations (open, save)
- [x] Keyboard input handling
- [x] Configuration system

### Phase 2: Core Features 🚧
- [ ] Syntax highlighting integration
- [ ] Multi-cursor editing
- [ ] Search and replace
- [ ] File explorer sidebar
- [ ] Tab management
- [ ] Theme system (dark/light)
- [ ] Custom keybindings

### Phase 3: Advanced Features 📋
- [ ] LSP integration
- [ ] Code folding
- [ ] Git integration
- [ ] Integrated terminal
- [ ] Plugin system
- [ ] Extension marketplace

### Phase 4: Polish 📋
- [ ] Performance optimization
- [ ] Memory profiling
- [ ] Accessibility
- [ ] Localization
- [ ] Documentation

## Tech Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Language | Rust | Safety, performance, concurrency |
| UI Framework | iced | Modern, reactive, pure Rust |
| Text Buffer | ropey | Efficient rope data structure |
| Parsing | tree-sitter | Incremental syntax highlighting |
| Async | tokio | File I/O, LSP communication |
| LSP | lsp-types | Language server integration |

## Key Design Decisions

### Why Rope for Text Buffer?

The rope data structure provides O(log n) insertion and deletion operations, making it ideal for text editors handling large files. Unlike gap buffers, ropes also enable efficient undo/redo through structural sharing.

### Why iced for UI?

- **Pure Rust**: No FFI overhead or binding maintenance
- **Elm Architecture**: Predictable state management
- **Cross-Platform**: Native look on all platforms
- **Performance**: GPU-accelerated rendering

### Why Tree-sitter for Syntax?

- **Incremental**: Only re-parses changed portions
- **Error-Tolerant**: Valid trees even with syntax errors
- **Accurate**: Real parsing, not regex heuristics
- **Fast**: Written in C with Rust bindings

## Learning Resources

This codebase is designed to be educational. Key Rust concepts demonstrated:

- **Ownership & Borrowing**: See `luminex-buffer/src/buffer.rs`
- **Traits & Generics**: See `luminex-core/src/command.rs`
- **Error Handling**: See `luminex-core/src/lib.rs`
- **Async/Await**: See `luminex-lsp/src/lib.rs`
- **FFI**: See `luminex-syntax/src/lib.rs`
- **Module Organization**: See project structure

## Contributing

Contributions are welcome! Please read our contributing guidelines before submitting PRs.

## License

MIT License - see LICENSE file for details.

## Acknowledgments

Inspired by:
- [Zed](https://zed.dev) - High-performance editor
- [Helix](https://helix-editor.com) - Post-modern modal editor
- [Lapce](https://lapce.dev) - Lightning-fast editor
- [VS Code](https://code.visualstudio.com) - Feature-rich IDE
# luminux-code
