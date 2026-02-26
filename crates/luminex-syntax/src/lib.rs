//! # Luminex Syntax
//!
//! Syntax highlighting using tree-sitter for incremental parsing.
//!
//! ## Why Tree-sitter?
//!
//! Tree-sitter is a parser generator tool and incremental parsing library:
//! - **Incremental**: Only re-parses changed portions of the code
//! - **Error-tolerant**: Produces valid syntax trees even with errors
//! - **Fast**: Written in C with Rust bindings
//! - **Accurate**: Real parsing, not regex-based highlighting
//!
//! ## Learning: FFI (Foreign Function Interface)
//!
//! Tree-sitter is written in C. Rust's FFI allows calling C functions:
//! - `extern "C"` blocks declare C functions
//! - `unsafe` blocks required for calling them
//! - The `tree-sitter` crate provides safe wrappers

use std::collections::HashMap;
use tree_sitter::{Language, Node, Parser, Tree};

/// Errors that can occur during syntax highlighting.
#[derive(Debug, thiserror::Error)]
pub enum SyntaxError {
    #[error("Unknown language: {0}")]
    UnknownLanguage(String),

    #[error("Parser error")]
    ParseError,

    #[error("Query error: {0}")]
    QueryError(String),
}

/// A syntax highlighter for a specific language.
pub struct Highlighter {
    parser: Parser,
    #[allow(dead_code)]
    language: Language,
    tree: Option<Tree>,
}

impl Highlighter {
    /// Creates a new highlighter for a language.
    pub fn new(lang: &str) -> Result<Self, SyntaxError> {
        let language = get_language(lang)?;

        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| SyntaxError::ParseError)?;

        Ok(Self {
            parser,
            language,
            tree: None,
        })
    }

    /// Parses source code and returns the syntax tree.
    pub fn parse(&mut self, source: &str) -> Result<(), SyntaxError> {
        let tree = self
            .parser
            .parse(source, self.tree.as_ref())
            .ok_or(SyntaxError::ParseError)?;
        self.tree = Some(tree);
        Ok(())
    }

    /// Parses with an edit (for incremental updates).
    pub fn parse_with_edit(
        &mut self,
        source: &str,
        edit: tree_sitter::InputEdit,
    ) -> Result<(), SyntaxError> {
        if let Some(tree) = &mut self.tree {
            tree.edit(&edit);
        }
        self.parse(source)
    }

    /// Returns syntax highlights for the source code.
    pub fn highlight(&self, _source: &[u8]) -> Vec<HighlightSpan> {
        let tree = match &self.tree {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut spans = Vec::new();
        self.collect_highlights(tree.root_node(), &mut spans);
        spans
    }

    /// Recursively collects highlights from the syntax tree.
    fn collect_highlights(&self, node: Node, spans: &mut Vec<HighlightSpan>) {
        let kind = highlight_kind_from_node(node.kind());

        if kind != HighlightKind::None {
            spans.push(HighlightSpan {
                start: node.start_byte(),
                end: node.end_byte(),
                kind,
            });
        }

        // Process children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_highlights(child, spans);
        }
    }

    /// Returns the syntax tree (for debugging).
    pub fn tree(&self) -> Option<&Tree> {
        self.tree.as_ref()
    }
}

/// A highlighted span of text.
#[derive(Debug, Clone)]
pub struct HighlightSpan {
    /// Start byte offset
    pub start: usize,
    /// End byte offset
    pub end: usize,
    /// Kind of syntax element
    pub kind: HighlightKind,
}

/// Types of syntax elements for highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightKind {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
    Variable,
    Constant,
    Operator,
    Punctuation,
    Attribute,
    Tag,
    Property,
    Parameter,
    Label,
    Namespace,
    Error,
    None,
}

impl HighlightKind {
    /// Returns the theme color key for this kind.
    pub fn theme_key(&self) -> &'static str {
        match self {
            HighlightKind::Keyword => "keyword",
            HighlightKind::String => "string",
            HighlightKind::Number => "number",
            HighlightKind::Comment => "comment",
            HighlightKind::Function => "function",
            HighlightKind::Type => "type_name",
            HighlightKind::Variable => "variable",
            HighlightKind::Constant => "constant",
            HighlightKind::Operator => "operator",
            HighlightKind::Punctuation => "punctuation",
            HighlightKind::Attribute => "attribute",
            HighlightKind::Tag => "tag",
            HighlightKind::Property => "variable",
            HighlightKind::Parameter => "variable",
            HighlightKind::Label => "constant",
            HighlightKind::Namespace => "type_name",
            HighlightKind::Error => "keyword",
            HighlightKind::None => "variable",
        }
    }
}

/// Maps node kinds to highlight kinds based on common tree-sitter node types.
fn highlight_kind_from_node(kind: &str) -> HighlightKind {
    match kind {
        // Keywords (common across languages)
        "fn" | "let" | "mut" | "const" | "static" | "pub" | "use" | "mod" | "struct"
        | "enum" | "impl" | "trait" | "type" | "where" | "if" | "else" | "match" | "for"
        | "while" | "loop" | "break" | "continue" | "return" | "async" | "await"
        | "unsafe" | "extern" | "crate" | "self" | "super" | "as" | "in" | "ref"
        | "move" | "dyn" | "true" | "false" | "function" | "class" | "def" | "import"
        | "from" | "try" | "except" | "finally" | "with" | "yield" | "lambda"
        | "var" | "new" | "delete" | "typeof" | "instanceof" | "void" | "throw"
        | "catch" | "switch" | "case" | "default" | "export" | "extends" => {
            HighlightKind::Keyword
        }

        // Strings
        "string_literal" | "raw_string_literal" | "char_literal" | "string"
        | "template_string" | "string_content" => HighlightKind::String,

        // Numbers
        "integer_literal" | "float_literal" | "number" | "integer" | "float" => {
            HighlightKind::Number
        }

        // Comments
        "line_comment" | "block_comment" | "comment" => HighlightKind::Comment,

        // Functions
        "function_item" | "function_definition" | "method_definition"
        | "call_expression" | "macro_invocation" => HighlightKind::Function,

        // Types
        "type_identifier" | "primitive_type" | "generic_type" | "scoped_type_identifier" => {
            HighlightKind::Type
        }

        // Attributes/Decorators
        "attribute_item" | "attribute" | "decorator" => HighlightKind::Attribute,

        // Operators
        "binary_expression" | "unary_expression" | "assignment_expression" => {
            HighlightKind::Operator
        }

        // Everything else
        _ => HighlightKind::None,
    }
}

/// Gets the tree-sitter language.
fn get_language(lang: &str) -> Result<Language, SyntaxError> {
    match lang {
        "rust" | "rs" => Ok(tree_sitter_rust::LANGUAGE.into()),
        "javascript" | "js" | "jsx" => Ok(tree_sitter_javascript::LANGUAGE.into()),
        "python" | "py" => Ok(tree_sitter_python::LANGUAGE.into()),
        "json" => Ok(tree_sitter_json::LANGUAGE.into()),
        _ => Err(SyntaxError::UnknownLanguage(lang.to_string())),
    }
}

/// Language registry for managing multiple highlighters.
pub struct LanguageRegistry {
    highlighters: HashMap<String, Highlighter>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            highlighters: HashMap::new(),
        }
    }

    /// Gets or creates a highlighter for a language.
    pub fn get_mut(&mut self, lang: &str) -> Result<&mut Highlighter, SyntaxError> {
        if !self.highlighters.contains_key(lang) {
            let highlighter = Highlighter::new(lang)?;
            self.highlighters.insert(lang.to_string(), highlighter);
        }
        Ok(self.highlighters.get_mut(lang).unwrap())
    }

    /// Returns supported languages.
    pub fn supported_languages() -> &'static [&'static str] {
        &["rust", "javascript", "python", "json"]
    }
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_highlighting() {
        let mut highlighter = Highlighter::new("rust").unwrap();
        let source = r#"fn main() {
    println!("Hello, world!");
}"#;

        highlighter.parse(source).unwrap();
        let spans = highlighter.highlight(source.as_bytes());

        assert!(!spans.is_empty());
    }

    #[test]
    fn test_unknown_language() {
        let result = Highlighter::new("unknown_lang");
        assert!(result.is_err());
    }
}
