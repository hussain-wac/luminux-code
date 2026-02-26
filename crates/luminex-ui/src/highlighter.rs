//! Syntax highlighting integration for the editor.
//!
//! Provides multi-language token-level syntax highlighting using pattern matching.

use iced::advanced::text::highlighter::{Format, Highlighter};
use iced::{Color, Font};
use std::ops::Range;

/// Colors for syntax highlighting (dark theme)
mod colors {
    use iced::Color;

    pub const KEYWORD: Color = Color::from_rgb(0.86, 0.55, 0.76);     // Pink/purple
    pub const CONTROL: Color = Color::from_rgb(0.86, 0.55, 0.76);     // Same as keyword for control flow
    pub const STRING: Color = Color::from_rgb(0.72, 0.84, 0.55);      // Green
    pub const NUMBER: Color = Color::from_rgb(0.82, 0.68, 0.55);      // Orange
    pub const COMMENT: Color = Color::from_rgb(0.50, 0.55, 0.55);     // Gray
    pub const FUNCTION: Color = Color::from_rgb(0.55, 0.75, 0.90);    // Blue
    pub const TYPE: Color = Color::from_rgb(0.90, 0.80, 0.55);        // Yellow
    pub const VARIABLE: Color = Color::from_rgb(0.85, 0.85, 0.85);    // Light gray
    pub const OPERATOR: Color = Color::from_rgb(0.80, 0.80, 0.90);    // Light blue-gray
    pub const PUNCTUATION: Color = Color::from_rgb(0.70, 0.70, 0.70); // Gray
    pub const ATTRIBUTE: Color = Color::from_rgb(0.90, 0.80, 0.55);   // Yellow
    pub const CONSTANT: Color = Color::from_rgb(0.90, 0.60, 0.50);    // Red-orange
    pub const TAG: Color = Color::from_rgb(0.80, 0.50, 0.50);         // Red
    pub const PROPERTY: Color = Color::from_rgb(0.55, 0.75, 0.90);    // Blue
    pub const BOOLEAN: Color = Color::from_rgb(0.82, 0.68, 0.55);     // Orange (same as number)
    pub const MACRO: Color = Color::from_rgb(0.55, 0.80, 0.80);       // Cyan
    pub const LIFETIME: Color = Color::from_rgb(0.90, 0.70, 0.55);    // Light orange
    pub const DEFAULT: Color = Color::from_rgb(0.90, 0.90, 0.90);     // White-ish
}

/// Settings for the highlighter.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HighlightSettings {
    pub language: String,
}

/// Format for highlighted text.
#[derive(Debug, Clone, Copy)]
pub struct HighlightFormat {
    pub color: Color,
}

impl HighlightFormat {
    pub fn to_format(self, _font: Font) -> Format<Font> {
        Format {
            color: Some(self.color),
            font: None,
        }
    }
}

/// Token kind used internally for classification.
#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenKind {
    Keyword,
    Control,
    Type,
    Function,
    String,
    Number,
    Comment,
    Operator,
    Punctuation,
    Attribute,
    Constant,
    Boolean,
    Macro,
    Lifetime,
    Tag,
    Property,
    Variable,
    Default,
}

impl TokenKind {
    fn color(self) -> Color {
        match self {
            Self::Keyword => colors::KEYWORD,
            Self::Control => colors::CONTROL,
            Self::Type => colors::TYPE,
            Self::Function => colors::FUNCTION,
            Self::String => colors::STRING,
            Self::Number => colors::NUMBER,
            Self::Comment => colors::COMMENT,
            Self::Operator => colors::OPERATOR,
            Self::Punctuation => colors::PUNCTUATION,
            Self::Attribute => colors::ATTRIBUTE,
            Self::Constant => colors::CONSTANT,
            Self::Boolean => colors::BOOLEAN,
            Self::Macro => colors::MACRO,
            Self::Lifetime => colors::LIFETIME,
            Self::Tag => colors::TAG,
            Self::Property => colors::PROPERTY,
            Self::Variable => colors::VARIABLE,
            Self::Default => colors::DEFAULT,
        }
    }
}

// ============================================================================
// Language keyword sets
// ============================================================================

fn rust_keywords() -> &'static [&'static str] {
    &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn",
        "else", "enum", "extern", "fn", "for", "if", "impl", "in", "let",
        "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
        "self", "Self", "static", "struct", "super", "trait", "type",
        "unsafe", "use", "where", "while", "yield", "macro_rules",
    ]
}

fn rust_control() -> &'static [&'static str] {
    &[
        "if", "else", "for", "while", "loop", "match", "break", "continue",
        "return", "yield", "async", "await",
    ]
}

fn rust_types() -> &'static [&'static str] {
    &[
        "bool", "char", "f32", "f64", "i8", "i16", "i32", "i64", "i128",
        "isize", "str", "u8", "u16", "u32", "u64", "u128", "usize",
        "String", "Vec", "Option", "Result", "Box", "Rc", "Arc",
        "HashMap", "HashSet", "BTreeMap", "BTreeSet", "VecDeque",
        "PathBuf", "Path", "Cow", "Cell", "RefCell", "Mutex", "RwLock",
        "Pin", "Future", "Stream", "Iterator", "Display", "Debug",
        "Clone", "Copy", "Send", "Sync", "Sized", "Drop", "Fn", "FnMut",
        "FnOnce", "Default", "From", "Into", "TryFrom", "TryInto",
        "AsRef", "AsMut", "Deref", "DerefMut", "Borrow", "ToOwned",
        "ToString", "Ord", "PartialOrd", "Eq", "PartialEq", "Hash",
    ]
}

fn rust_constants() -> &'static [&'static str] {
    &["true", "false", "None", "Some", "Ok", "Err"]
}

fn python_keywords() -> &'static [&'static str] {
    &[
        "and", "as", "assert", "async", "await", "break", "class", "continue",
        "def", "del", "elif", "else", "except", "finally", "for", "from",
        "global", "if", "import", "in", "is", "lambda", "nonlocal", "not",
        "or", "pass", "raise", "return", "try", "while", "with", "yield",
    ]
}

fn python_control() -> &'static [&'static str] {
    &[
        "if", "elif", "else", "for", "while", "break", "continue", "return",
        "try", "except", "finally", "raise", "with", "yield", "async", "await",
    ]
}

fn python_types() -> &'static [&'static str] {
    &[
        "int", "float", "str", "bool", "list", "dict", "tuple", "set",
        "frozenset", "bytes", "bytearray", "memoryview", "range", "complex",
        "type", "object", "None", "Ellipsis", "NotImplemented",
    ]
}

fn python_builtins() -> &'static [&'static str] {
    &[
        "print", "len", "range", "enumerate", "zip", "map", "filter",
        "sorted", "reversed", "min", "max", "sum", "abs", "round",
        "isinstance", "issubclass", "hasattr", "getattr", "setattr",
        "delattr", "super", "property", "staticmethod", "classmethod",
        "open", "input", "repr", "format", "id", "hash", "iter", "next",
        "any", "all", "dir", "vars", "globals", "locals", "type",
    ]
}

fn python_constants() -> &'static [&'static str] {
    &["True", "False", "None"]
}

fn js_keywords() -> &'static [&'static str] {
    &[
        "async", "await", "break", "case", "catch", "class", "const",
        "continue", "debugger", "default", "delete", "do", "else", "export",
        "extends", "finally", "for", "from", "function", "if", "import",
        "in", "instanceof", "let", "new", "of", "return", "static", "super",
        "switch", "this", "throw", "try", "typeof", "var", "void", "while",
        "with", "yield", "enum", "implements", "interface", "package",
        "private", "protected", "public", "abstract", "as", "type",
    ]
}

fn js_control() -> &'static [&'static str] {
    &[
        "if", "else", "for", "while", "do", "switch", "case", "break",
        "continue", "return", "throw", "try", "catch", "finally",
        "async", "await", "yield",
    ]
}

fn js_types() -> &'static [&'static str] {
    &[
        "Array", "Object", "String", "Number", "Boolean", "Function",
        "Symbol", "BigInt", "Map", "Set", "WeakMap", "WeakSet",
        "Promise", "Proxy", "Reflect", "RegExp", "Error", "TypeError",
        "RangeError", "ReferenceError", "SyntaxError", "Date", "JSON",
        "Math", "Intl", "ArrayBuffer", "DataView", "Float32Array",
        "Float64Array", "Int8Array", "Int16Array", "Int32Array",
        "Uint8Array", "Uint16Array", "Uint32Array",
        // TS types
        "string", "number", "boolean", "any", "void", "never",
        "unknown", "undefined", "null", "object", "symbol", "bigint",
    ]
}

fn js_constants() -> &'static [&'static str] {
    &["true", "false", "null", "undefined", "NaN", "Infinity", "this"]
}

fn go_keywords() -> &'static [&'static str] {
    &[
        "break", "case", "chan", "const", "continue", "default", "defer",
        "else", "fallthrough", "for", "func", "go", "goto", "if", "import",
        "interface", "map", "package", "range", "return", "select", "struct",
        "switch", "type", "var",
    ]
}

fn go_types() -> &'static [&'static str] {
    &[
        "bool", "byte", "complex64", "complex128", "error", "float32",
        "float64", "int", "int8", "int16", "int32", "int64", "rune",
        "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
    ]
}

fn go_constants() -> &'static [&'static str] {
    &["true", "false", "nil", "iota"]
}

fn java_keywords() -> &'static [&'static str] {
    &[
        "abstract", "assert", "break", "case", "catch", "class", "const",
        "continue", "default", "do", "else", "enum", "extends", "final",
        "finally", "for", "goto", "if", "implements", "import", "instanceof",
        "interface", "native", "new", "package", "private", "protected",
        "public", "return", "static", "strictfp", "super", "switch",
        "synchronized", "this", "throw", "throws", "transient", "try",
        "void", "volatile", "while", "var", "yield", "record", "sealed",
        "permits", "non-sealed",
    ]
}

fn java_types() -> &'static [&'static str] {
    &[
        "boolean", "byte", "char", "double", "float", "int", "long", "short",
        "String", "Integer", "Long", "Double", "Float", "Boolean", "Byte",
        "Character", "Short", "Object", "Class", "Void",
        "List", "ArrayList", "LinkedList", "Map", "HashMap", "TreeMap",
        "Set", "HashSet", "TreeSet", "Queue", "Stack", "Vector",
        "Optional", "Stream", "Collection", "Collections", "Arrays",
        "Iterable", "Iterator", "Comparable", "Comparator", "Runnable",
        "Callable", "Future", "CompletableFuture", "Thread", "Exception",
        "RuntimeException", "Throwable", "Error",
    ]
}

fn java_constants() -> &'static [&'static str] {
    &["true", "false", "null", "this", "super"]
}

fn c_keywords() -> &'static [&'static str] {
    &[
        "auto", "break", "case", "char", "const", "continue", "default",
        "do", "double", "else", "enum", "extern", "float", "for", "goto",
        "if", "inline", "int", "long", "register", "restrict", "return",
        "short", "signed", "sizeof", "static", "struct", "switch",
        "typedef", "union", "unsigned", "void", "volatile", "while",
        "_Alignas", "_Alignof", "_Atomic", "_Bool", "_Complex",
        "_Generic", "_Imaginary", "_Noreturn", "_Static_assert",
        "_Thread_local",
    ]
}

fn cpp_keywords() -> &'static [&'static str] {
    &[
        "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand",
        "bitor", "bool", "break", "case", "catch", "char", "char8_t",
        "char16_t", "char32_t", "class", "compl", "concept", "const",
        "consteval", "constexpr", "constinit", "const_cast", "continue",
        "co_await", "co_return", "co_yield", "decltype", "default",
        "delete", "do", "double", "dynamic_cast", "else", "enum",
        "explicit", "export", "extern", "false", "float", "for", "friend",
        "goto", "if", "inline", "int", "long", "mutable", "namespace",
        "new", "noexcept", "not", "not_eq", "nullptr", "operator", "or",
        "or_eq", "private", "protected", "public", "register",
        "reinterpret_cast", "requires", "return", "short", "signed",
        "sizeof", "static", "static_assert", "static_cast", "struct",
        "switch", "template", "this", "thread_local", "throw", "true",
        "try", "typedef", "typeid", "typename", "union", "unsigned",
        "using", "virtual", "void", "volatile", "wchar_t", "while",
        "xor", "xor_eq", "override", "final",
    ]
}

fn cpp_types() -> &'static [&'static str] {
    &[
        "string", "vector", "map", "set", "unordered_map", "unordered_set",
        "array", "list", "deque", "queue", "stack", "pair", "tuple",
        "optional", "variant", "any", "shared_ptr", "unique_ptr",
        "weak_ptr", "function", "thread", "mutex", "atomic",
        "size_t", "int8_t", "int16_t", "int32_t", "int64_t",
        "uint8_t", "uint16_t", "uint32_t", "uint64_t", "ptrdiff_t",
    ]
}

fn ruby_keywords() -> &'static [&'static str] {
    &[
        "alias", "and", "begin", "break", "case", "class", "def", "defined?",
        "do", "else", "elsif", "end", "ensure", "for", "if", "in",
        "module", "next", "nil", "not", "or", "redo", "rescue", "retry",
        "return", "self", "super", "then", "undef", "unless", "until",
        "when", "while", "yield", "require", "include", "extend", "attr_reader",
        "attr_writer", "attr_accessor", "private", "protected", "public",
        "raise", "lambda", "proc",
    ]
}

fn ruby_constants() -> &'static [&'static str] {
    &["true", "false", "nil", "self", "__FILE__", "__LINE__", "__dir__"]
}

fn php_keywords() -> &'static [&'static str] {
    &[
        "abstract", "and", "array", "as", "break", "callable", "case",
        "catch", "class", "clone", "const", "continue", "declare",
        "default", "die", "do", "echo", "else", "elseif", "empty",
        "enddeclare", "endfor", "endforeach", "endif", "endswitch",
        "endwhile", "eval", "exit", "extends", "final", "finally",
        "fn", "for", "foreach", "function", "global", "goto", "if",
        "implements", "include", "include_once", "instanceof", "insteadof",
        "interface", "isset", "list", "match", "namespace", "new", "or",
        "print", "private", "protected", "public", "readonly", "require",
        "require_once", "return", "static", "switch", "throw", "trait",
        "try", "unset", "use", "var", "while", "xor", "yield",
    ]
}

fn php_constants() -> &'static [&'static str] {
    &["true", "false", "null", "TRUE", "FALSE", "NULL", "self", "parent", "static"]
}

fn swift_keywords() -> &'static [&'static str] {
    &[
        "associatedtype", "break", "case", "catch", "class", "continue",
        "convenience", "default", "defer", "deinit", "do", "else", "enum",
        "extension", "fallthrough", "fileprivate", "final", "for", "func",
        "guard", "if", "import", "in", "init", "inout", "internal", "is",
        "lazy", "let", "mutating", "nil", "nonmutating", "open", "operator",
        "optional", "override", "private", "protocol", "public", "repeat",
        "required", "rethrows", "return", "self", "Self", "some", "static",
        "struct", "subscript", "super", "switch", "throw", "throws", "try",
        "typealias", "unowned", "var", "weak", "where", "while", "async",
        "await", "actor",
    ]
}

fn swift_types() -> &'static [&'static str] {
    &[
        "Int", "Int8", "Int16", "Int32", "Int64", "UInt", "UInt8",
        "UInt16", "UInt32", "UInt64", "Float", "Double", "Bool",
        "String", "Character", "Array", "Dictionary", "Set", "Optional",
        "Any", "AnyObject", "Void", "Never", "Result", "Error",
    ]
}

fn kotlin_keywords() -> &'static [&'static str] {
    &[
        "abstract", "actual", "annotation", "as", "break", "by", "catch",
        "class", "companion", "const", "constructor", "continue", "crossinline",
        "data", "delegate", "do", "dynamic", "else", "enum", "expect",
        "external", "final", "finally", "for", "fun", "get", "if",
        "import", "in", "infix", "init", "inline", "inner", "interface",
        "internal", "is", "lateinit", "noinline", "object", "open",
        "operator", "out", "override", "package", "private", "protected",
        "public", "reified", "return", "sealed", "set", "super",
        "suspend", "tailrec", "this", "throw", "try", "typealias",
        "val", "var", "vararg", "when", "where", "while",
    ]
}

fn kotlin_constants() -> &'static [&'static str] {
    &["true", "false", "null", "this", "super", "it"]
}

fn css_keywords() -> &'static [&'static str] {
    &[
        "!important", "@media", "@import", "@font-face", "@keyframes",
        "@charset", "@supports", "@namespace", "@page", "@property",
        "@layer", "@container",
    ]
}

fn css_properties() -> &'static [&'static str] {
    &[
        "color", "background", "background-color", "background-image",
        "border", "border-radius", "margin", "padding", "width", "height",
        "display", "position", "top", "right", "bottom", "left",
        "font", "font-size", "font-weight", "font-family", "font-style",
        "text-align", "text-decoration", "text-transform", "line-height",
        "letter-spacing", "word-spacing", "overflow", "opacity", "z-index",
        "flex", "flex-direction", "flex-wrap", "justify-content",
        "align-items", "align-content", "gap", "grid", "grid-template",
        "transform", "transition", "animation", "box-shadow", "cursor",
        "visibility", "content", "list-style", "outline", "max-width",
        "max-height", "min-width", "min-height", "float", "clear",
        "white-space", "vertical-align", "box-sizing",
    ]
}

fn shell_keywords() -> &'static [&'static str] {
    &[
        "if", "then", "else", "elif", "fi", "case", "esac", "for", "while",
        "until", "do", "done", "in", "function", "select", "time", "coproc",
        "return", "exit", "break", "continue", "shift", "export", "readonly",
        "declare", "local", "typeset", "unset", "source", "alias", "eval",
        "exec", "trap", "set",
    ]
}

fn shell_builtins() -> &'static [&'static str] {
    &[
        "echo", "printf", "read", "cd", "pwd", "ls", "cp", "mv", "rm",
        "mkdir", "rmdir", "cat", "grep", "sed", "awk", "find", "sort",
        "cut", "tr", "wc", "head", "tail", "chmod", "chown", "test",
        "true", "false",
    ]
}

fn sql_keywords() -> &'static [&'static str] {
    &[
        "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "INSERT", "INTO",
        "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "ALTER",
        "DROP", "INDEX", "VIEW", "JOIN", "INNER", "LEFT", "RIGHT", "OUTER",
        "FULL", "ON", "AS", "ORDER", "BY", "GROUP", "HAVING", "LIMIT",
        "OFFSET", "UNION", "ALL", "DISTINCT", "EXISTS", "IN", "BETWEEN",
        "LIKE", "IS", "NULL", "TRUE", "FALSE", "CASE", "WHEN", "THEN",
        "ELSE", "END", "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
        "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "CONSTRAINT", "UNIQUE",
        "CHECK", "DEFAULT", "AUTO_INCREMENT", "CASCADE", "TRIGGER",
        "PROCEDURE", "FUNCTION", "RETURNS", "DECLARE", "CURSOR",
        // lowercase variants
        "select", "from", "where", "and", "or", "not", "insert", "into",
        "values", "update", "set", "delete", "create", "table", "alter",
        "drop", "index", "view", "join", "inner", "left", "right", "outer",
        "full", "on", "as", "order", "by", "group", "having", "limit",
        "offset", "union", "all", "distinct", "exists", "in", "between",
        "like", "is", "null", "true", "false", "case", "when", "then",
        "else", "end", "begin", "commit", "rollback", "primary", "key",
        "foreign", "references",
    ]
}

fn sql_types() -> &'static [&'static str] {
    &[
        "INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "FLOAT",
        "DOUBLE", "DECIMAL", "NUMERIC", "VARCHAR", "CHAR", "TEXT",
        "BLOB", "DATE", "TIME", "DATETIME", "TIMESTAMP", "BOOLEAN",
        "SERIAL", "UUID",
        "int", "integer", "bigint", "smallint", "float", "double",
        "decimal", "numeric", "varchar", "char", "text", "blob",
        "date", "time", "datetime", "timestamp", "boolean", "serial",
    ]
}

fn lua_keywords() -> &'static [&'static str] {
    &[
        "and", "break", "do", "else", "elseif", "end", "for", "function",
        "goto", "if", "in", "local", "not", "or", "repeat", "return",
        "then", "until", "while",
    ]
}

fn lua_constants() -> &'static [&'static str] {
    &["true", "false", "nil"]
}

fn toml_keywords() -> &'static [&'static str] {
    &["true", "false"]
}

fn yaml_keywords() -> &'static [&'static str] {
    &["true", "false", "null", "yes", "no", "on", "off", "True", "False", "Null"]
}

fn dart_keywords() -> &'static [&'static str] {
    &[
        "abstract", "as", "assert", "async", "await", "break", "case",
        "catch", "class", "const", "continue", "covariant", "default",
        "deferred", "do", "dynamic", "else", "enum", "export", "extends",
        "extension", "external", "factory", "final", "finally", "for",
        "Function", "get", "hide", "if", "implements", "import", "in",
        "interface", "is", "late", "library", "mixin", "new", "null",
        "on", "operator", "part", "required", "rethrow", "return",
        "sealed", "set", "show", "static", "super", "switch", "sync",
        "this", "throw", "try", "typedef", "var", "void", "when",
        "while", "with", "yield",
    ]
}

fn dart_types() -> &'static [&'static str] {
    &[
        "int", "double", "num", "String", "bool", "List", "Map", "Set",
        "Future", "Stream", "Iterable", "Iterator", "Object", "dynamic",
        "void", "Never", "Null", "Type", "Symbol", "Function",
    ]
}

fn dart_constants() -> &'static [&'static str] {
    &["true", "false", "null", "this", "super"]
}

fn csharp_keywords() -> &'static [&'static str] {
    &[
        "abstract", "as", "base", "bool", "break", "byte", "case", "catch",
        "char", "checked", "class", "const", "continue", "decimal", "default",
        "delegate", "do", "double", "else", "enum", "event", "explicit",
        "extern", "false", "finally", "fixed", "float", "for", "foreach",
        "goto", "if", "implicit", "in", "int", "interface", "internal",
        "is", "lock", "long", "namespace", "new", "null", "object",
        "operator", "out", "override", "params", "private", "protected",
        "public", "readonly", "record", "ref", "return", "sbyte", "sealed",
        "short", "sizeof", "stackalloc", "static", "string", "struct",
        "switch", "this", "throw", "true", "try", "typeof", "uint",
        "ulong", "unchecked", "unsafe", "ushort", "using", "var",
        "virtual", "void", "volatile", "while", "async", "await",
        "yield", "partial", "where", "get", "set", "init", "value",
        "add", "remove", "global", "when", "with",
    ]
}

// ============================================================================
// Language configuration
// ============================================================================

struct LangConfig {
    keywords: &'static [&'static str],
    control: &'static [&'static str],
    types: &'static [&'static str],
    constants: &'static [&'static str],
    builtins: &'static [&'static str],
    line_comment: &'static str,
    block_comment_start: &'static str,
    block_comment_end: &'static str,
    has_single_quote_strings: bool,
    has_backtick_strings: bool,
    has_triple_quote_strings: bool,
    has_hash_comments: bool,
    has_double_dash_comments: bool,
    has_lifetimes: bool,
    has_macros: bool,
    has_attributes: bool,
    has_dollar_vars: bool,
}

impl Default for LangConfig {
    fn default() -> Self {
        Self {
            keywords: &[],
            control: &[],
            types: &[],
            constants: &[],
            builtins: &[],
            line_comment: "//",
            block_comment_start: "/*",
            block_comment_end: "*/",
            has_single_quote_strings: true,
            has_backtick_strings: false,
            has_triple_quote_strings: false,
            has_hash_comments: false,
            has_double_dash_comments: false,
            has_lifetimes: false,
            has_macros: false,
            has_attributes: false,
            has_dollar_vars: false,
        }
    }
}

fn get_lang_config(language: &str) -> LangConfig {
    match language {
        "rust" => LangConfig {
            keywords: rust_keywords(),
            control: rust_control(),
            types: rust_types(),
            constants: rust_constants(),
            builtins: &[],
            has_lifetimes: true,
            has_macros: true,
            has_attributes: true,
            ..Default::default()
        },
        "python" => LangConfig {
            keywords: python_keywords(),
            control: python_control(),
            types: python_types(),
            constants: python_constants(),
            builtins: python_builtins(),
            line_comment: "#",
            block_comment_start: "\"\"\"",
            block_comment_end: "\"\"\"",
            has_hash_comments: true,
            has_single_quote_strings: true,
            has_triple_quote_strings: true,
            ..Default::default()
        },
        "javascript" | "typescript" | "jsx" | "tsx" => LangConfig {
            keywords: js_keywords(),
            control: js_control(),
            types: js_types(),
            constants: js_constants(),
            builtins: &[],
            has_backtick_strings: true,
            ..Default::default()
        },
        "go" => LangConfig {
            keywords: go_keywords(),
            control: &["if", "else", "for", "switch", "case", "break", "continue", "return", "goto", "select", "defer", "go"],
            types: go_types(),
            constants: go_constants(),
            builtins: &["append", "cap", "close", "copy", "delete", "len", "make", "new", "panic", "print", "println", "recover"],
            has_backtick_strings: true,
            ..Default::default()
        },
        "java" => LangConfig {
            keywords: java_keywords(),
            control: &["if", "else", "for", "while", "do", "switch", "case", "break", "continue", "return", "throw", "try", "catch", "finally"],
            types: java_types(),
            constants: java_constants(),
            builtins: &[],
            has_attributes: true,
            ..Default::default()
        },
        "c" => LangConfig {
            keywords: c_keywords(),
            control: &["if", "else", "for", "while", "do", "switch", "case", "break", "continue", "return", "goto"],
            types: &["int", "char", "float", "double", "void", "long", "short", "unsigned", "signed", "size_t", "FILE", "NULL"],
            constants: &["NULL", "true", "false", "EOF", "stdin", "stdout", "stderr"],
            builtins: &["printf", "scanf", "malloc", "free", "calloc", "realloc", "strlen", "strcmp", "strcpy", "memcpy", "memset"],
            has_attributes: false,
            ..Default::default()
        },
        "cpp" | "c++" | "cxx" => LangConfig {
            keywords: cpp_keywords(),
            control: &["if", "else", "for", "while", "do", "switch", "case", "break", "continue", "return", "throw", "try", "catch", "goto"],
            types: cpp_types(),
            constants: &["true", "false", "nullptr", "NULL"],
            builtins: &["cout", "cin", "cerr", "endl", "std"],
            ..Default::default()
        },
        "ruby" => LangConfig {
            keywords: ruby_keywords(),
            control: &["if", "else", "elsif", "unless", "until", "while", "for", "do", "begin", "rescue", "ensure", "return", "break", "next", "redo", "retry", "case", "when"],
            types: &["Integer", "Float", "String", "Symbol", "Array", "Hash", "Regexp", "Range", "Proc", "Method", "Class", "Module", "IO", "File", "Dir", "Time"],
            constants: ruby_constants(),
            builtins: &["puts", "print", "p", "gets", "require", "require_relative", "include", "extend", "attr_reader", "attr_writer", "attr_accessor"],
            has_hash_comments: true,
            line_comment: "#",
            has_single_quote_strings: true,
            ..Default::default()
        },
        "php" => LangConfig {
            keywords: php_keywords(),
            control: &["if", "else", "elseif", "for", "foreach", "while", "do", "switch", "case", "break", "continue", "return", "throw", "try", "catch", "finally", "match"],
            types: &["int", "float", "string", "bool", "array", "object", "callable", "iterable", "void", "mixed", "never", "null"],
            constants: php_constants(),
            builtins: &["echo", "print", "isset", "unset", "empty", "die", "exit", "var_dump", "print_r", "array_push", "array_pop", "count", "strlen", "strpos", "substr"],
            has_hash_comments: true,
            has_dollar_vars: true,
            ..Default::default()
        },
        "swift" => LangConfig {
            keywords: swift_keywords(),
            control: &["if", "else", "for", "while", "repeat", "switch", "case", "break", "continue", "return", "throw", "do", "try", "catch", "guard", "defer"],
            types: swift_types(),
            constants: &["true", "false", "nil", "self", "Self"],
            builtins: &["print", "debugPrint", "dump", "fatalError", "precondition", "assert"],
            has_attributes: true,
            ..Default::default()
        },
        "kotlin" => LangConfig {
            keywords: kotlin_keywords(),
            control: &["if", "else", "for", "while", "do", "when", "break", "continue", "return", "throw", "try", "catch", "finally"],
            types: &["Int", "Long", "Short", "Byte", "Float", "Double", "Char", "Boolean", "String", "Any", "Unit", "Nothing", "Array", "List", "Map", "Set", "Pair", "Triple"],
            constants: kotlin_constants(),
            builtins: &["println", "print", "readLine", "listOf", "mapOf", "setOf", "arrayOf", "mutableListOf", "mutableMapOf", "mutableSetOf"],
            ..Default::default()
        },
        "css" | "scss" | "sass" => LangConfig {
            keywords: css_keywords(),
            control: &[],
            types: css_properties(),
            constants: &["inherit", "initial", "unset", "revert", "auto", "none", "block", "inline", "flex", "grid", "absolute", "relative", "fixed", "sticky"],
            builtins: &["rgb", "rgba", "hsl", "hsla", "calc", "var", "min", "max", "clamp", "url", "linear-gradient", "radial-gradient"],
            line_comment: "//",
            block_comment_start: "/*",
            block_comment_end: "*/",
            has_hash_comments: false,
            ..Default::default()
        },
        "html" | "htm" | "xml" | "svg" => LangConfig {
            keywords: &[],
            control: &[],
            types: &[],
            constants: &[],
            builtins: &[],
            line_comment: "",
            block_comment_start: "<!--",
            block_comment_end: "-->",
            has_single_quote_strings: true,
            ..Default::default()
        },
        "shell" | "bash" | "sh" | "zsh" => LangConfig {
            keywords: shell_keywords(),
            control: &["if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac"],
            types: &[],
            constants: &["true", "false"],
            builtins: shell_builtins(),
            line_comment: "#",
            has_hash_comments: true,
            has_single_quote_strings: true,
            has_backtick_strings: true,
            has_dollar_vars: true,
            ..Default::default()
        },
        "sql" => LangConfig {
            keywords: sql_keywords(),
            control: &["CASE", "WHEN", "THEN", "ELSE", "END", "IF", "case", "when", "then", "else", "end", "if"],
            types: sql_types(),
            constants: &["NULL", "TRUE", "FALSE", "null", "true", "false"],
            builtins: &["COUNT", "SUM", "AVG", "MIN", "MAX", "COALESCE", "CAST", "CONVERT", "IFNULL", "NULLIF", "count", "sum", "avg", "min", "max"],
            line_comment: "--",
            has_double_dash_comments: true,
            has_single_quote_strings: true,
            ..Default::default()
        },
        "lua" => LangConfig {
            keywords: lua_keywords(),
            control: &["if", "then", "else", "elseif", "for", "while", "repeat", "until", "do", "end", "return", "break", "goto"],
            types: &[],
            constants: lua_constants(),
            builtins: &["print", "type", "tostring", "tonumber", "pairs", "ipairs", "next", "select", "unpack", "require", "pcall", "xpcall", "error", "assert", "setmetatable", "getmetatable", "rawget", "rawset"],
            line_comment: "--",
            block_comment_start: "--[[",
            block_comment_end: "]]",
            has_double_dash_comments: true,
            has_single_quote_strings: true,
            ..Default::default()
        },
        "toml" => LangConfig {
            keywords: toml_keywords(),
            constants: toml_keywords(),
            line_comment: "#",
            has_hash_comments: true,
            has_single_quote_strings: true,
            has_triple_quote_strings: true,
            ..Default::default()
        },
        "yaml" | "yml" => LangConfig {
            keywords: yaml_keywords(),
            constants: yaml_keywords(),
            line_comment: "#",
            has_hash_comments: true,
            ..Default::default()
        },
        "json" => LangConfig {
            keywords: &[],
            constants: &["true", "false", "null"],
            line_comment: "",
            ..Default::default()
        },
        "markdown" | "md" => LangConfig {
            line_comment: "",
            ..Default::default()
        },
        "dart" => LangConfig {
            keywords: dart_keywords(),
            control: &["if", "else", "for", "while", "do", "switch", "case", "break", "continue", "return", "throw", "try", "catch", "finally", "rethrow"],
            types: dart_types(),
            constants: dart_constants(),
            builtins: &["print", "debugPrint"],
            has_single_quote_strings: true,
            has_triple_quote_strings: true,
            has_attributes: true,
            ..Default::default()
        },
        "csharp" | "cs" => LangConfig {
            keywords: csharp_keywords(),
            control: &["if", "else", "for", "foreach", "while", "do", "switch", "case", "break", "continue", "return", "throw", "try", "catch", "finally", "goto", "yield"],
            types: &["int", "long", "short", "byte", "float", "double", "decimal", "bool", "char", "string", "object", "void", "var", "dynamic",
                     "String", "Int32", "Int64", "Double", "Boolean", "List", "Dictionary", "HashSet", "Queue", "Stack", "Array", "Task", "Action", "Func"],
            constants: &["true", "false", "null", "this", "base"],
            builtins: &["Console", "Math", "Convert", "Environment", "GC", "Activator"],
            has_attributes: true,
            ..Default::default()
        },
        _ => LangConfig::default(),
    }
}

// ============================================================================
// Tokenizer
// ============================================================================

#[derive(Debug, Clone)]
struct Span {
    range: Range<usize>,
    kind: TokenKind,
}

/// State that persists across lines for multi-line constructs.
struct HighlighterState {
    in_block_comment: bool,
    in_multiline_string: bool,
    string_delim: char,
}

/// Syntax highlighter for the text editor.
pub struct EditorHighlighter {
    language: String,
    config: LangConfig,
    state: HighlighterState,
    spans: Vec<Span>,
    current_line_idx: usize,
}

impl Highlighter for EditorHighlighter {
    type Settings = HighlightSettings;
    type Highlight = HighlightFormat;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, HighlightFormat)> where Self: 'a;

    fn new(settings: &Self::Settings) -> Self {
        let config = get_lang_config(&settings.language);
        Self {
            language: settings.language.clone(),
            config,
            state: HighlighterState {
                in_block_comment: false,
                in_multiline_string: false,
                string_delim: '"',
            },
            spans: Vec::new(),
            current_line_idx: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        if self.language != new_settings.language {
            self.language = new_settings.language.clone();
            self.config = get_lang_config(&self.language);
            self.state.in_block_comment = false;
            self.state.in_multiline_string = false;
            self.current_line_idx = 0;
        }
    }

    fn change_line(&mut self, line: usize) {
        if line < self.current_line_idx {
            self.current_line_idx = line;
            // Reset multi-line state when going backwards
            self.state.in_block_comment = false;
            self.state.in_multiline_string = false;
        }
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        self.spans.clear();
        self.tokenize_line(line);
        self.current_line_idx += 1;

        if self.spans.is_empty() && !line.is_empty() {
            // Ensure at least one span covering the whole line
            self.spans.push(Span {
                range: 0..line.len(),
                kind: TokenKind::Default,
            });
        }

        self.spans
            .iter()
            .map(|s| {
                (
                    s.range.clone(),
                    HighlightFormat {
                        color: s.kind.color(),
                    },
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line_idx
    }
}

impl EditorHighlighter {
    fn tokenize_line(&mut self, line: &str) {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        // Handle continuing block comment from previous line
        if self.state.in_block_comment {
            let end = self.config.block_comment_end;
            if !end.is_empty() {
                if let Some(pos) = line.find(end) {
                    let end_pos = pos + end.len();
                    self.spans.push(Span {
                        range: 0..end_pos,
                        kind: TokenKind::Comment,
                    });
                    self.state.in_block_comment = false;
                    i = end_pos;
                } else {
                    // Entire line is inside block comment
                    self.spans.push(Span {
                        range: 0..len,
                        kind: TokenKind::Comment,
                    });
                    return;
                }
            }
        }

        // Handle continuing multi-line string from previous line
        if self.state.in_multiline_string {
            let delim = self.state.string_delim;
            let triple = format!("{}{}{}", delim, delim, delim);
            if self.config.has_triple_quote_strings {
                if let Some(pos) = line[i..].find(&triple) {
                    let end_pos = i + pos + 3;
                    self.spans.push(Span {
                        range: i..end_pos,
                        kind: TokenKind::String,
                    });
                    self.state.in_multiline_string = false;
                    i = end_pos;
                } else {
                    self.spans.push(Span {
                        range: i..len,
                        kind: TokenKind::String,
                    });
                    return;
                }
            }
        }

        while i < len {
            // Skip whitespace
            if bytes[i].is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Default,
                });
                continue;
            }

            // Check for line comments
            if !self.config.line_comment.is_empty() && line[i..].starts_with(self.config.line_comment) {
                self.spans.push(Span {
                    range: i..len,
                    kind: TokenKind::Comment,
                });
                return;
            }

            // Check for hash comments
            if self.config.has_hash_comments && bytes[i] == b'#' {
                // For CSS, # might be a color or selector, not a comment
                if self.language != "css" && self.language != "scss" {
                    self.spans.push(Span {
                        range: i..len,
                        kind: TokenKind::Comment,
                    });
                    return;
                }
            }

            // Check for double-dash comments
            if self.config.has_double_dash_comments && line[i..].starts_with("--") {
                // Check it's not block comment start (Lua: --[[)
                if self.language == "lua" && line[i..].starts_with("--[[") {
                    // Lua block comment
                    if let Some(pos) = line[i + 4..].find("]]") {
                        let end_pos = i + 4 + pos + 2;
                        self.spans.push(Span {
                            range: i..end_pos,
                            kind: TokenKind::Comment,
                        });
                        i = end_pos;
                        continue;
                    } else {
                        self.state.in_block_comment = true;
                        self.spans.push(Span {
                            range: i..len,
                            kind: TokenKind::Comment,
                        });
                        return;
                    }
                }
                self.spans.push(Span {
                    range: i..len,
                    kind: TokenKind::Comment,
                });
                return;
            }

            // Check for block comments
            if !self.config.block_comment_start.is_empty()
                && line[i..].starts_with(self.config.block_comment_start)
            {
                let start = i;
                let bcs_len = self.config.block_comment_start.len();
                let end_marker = self.config.block_comment_end;
                i += bcs_len;

                if let Some(pos) = line[i..].find(end_marker) {
                    let end_pos = i + pos + end_marker.len();
                    self.spans.push(Span {
                        range: start..end_pos,
                        kind: TokenKind::Comment,
                    });
                    i = end_pos;
                } else {
                    self.state.in_block_comment = true;
                    self.spans.push(Span {
                        range: start..len,
                        kind: TokenKind::Comment,
                    });
                    return;
                }
                continue;
            }

            // Rust attributes: #[...] or #![...]
            if self.config.has_attributes && bytes[i] == b'#' && i + 1 < len && (bytes[i + 1] == b'[' || (bytes[i + 1] == b'!' && i + 2 < len && bytes[i + 2] == b'[')) {
                let start = i;
                let mut depth = 0;
                while i < len {
                    if bytes[i] == b'[' {
                        depth += 1;
                    } else if bytes[i] == b']' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    i += 1;
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Attribute,
                });
                continue;
            }

            // Java/Kotlin/Swift/Dart annotations: @something
            if self.config.has_attributes && bytes[i] == b'@' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                if i > start + 1 {
                    self.spans.push(Span {
                        range: start..i,
                        kind: TokenKind::Attribute,
                    });
                    continue;
                }
            }

            // CSS at-rules
            if (self.language == "css" || self.language == "scss") && bytes[i] == b'@' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
                    i += 1;
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Keyword,
                });
                continue;
            }

            // Triple-quoted strings (Python, TOML, Dart)
            if self.config.has_triple_quote_strings && i + 2 < len {
                if (bytes[i] == b'"' && bytes[i + 1] == b'"' && bytes[i + 2] == b'"')
                    || (bytes[i] == b'\'' && bytes[i + 1] == b'\'' && bytes[i + 2] == b'\'')
                {
                    let delim = bytes[i] as char;
                    let triple = format!("{}{}{}", delim, delim, delim);
                    let start = i;
                    i += 3;
                    if let Some(pos) = line[i..].find(&triple) {
                        let end_pos = i + pos + 3;
                        self.spans.push(Span {
                            range: start..end_pos,
                            kind: TokenKind::String,
                        });
                        i = end_pos;
                    } else {
                        self.state.in_multiline_string = true;
                        self.state.string_delim = delim;
                        self.spans.push(Span {
                            range: start..len,
                            kind: TokenKind::String,
                        });
                        return;
                    }
                    continue;
                }
            }

            // Double-quoted strings
            if bytes[i] == b'"' {
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2; // skip escaped character
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::String,
                });
                continue;
            }

            // Single-quoted strings/chars
            if self.config.has_single_quote_strings && bytes[i] == b'\'' {
                // For Rust, single quotes are char literals or lifetimes
                if self.language == "rust" {
                    // Check for lifetime: 'a, 'static, etc.
                    if i + 1 < len && bytes[i + 1].is_ascii_alphabetic() {
                        let start = i;
                        i += 1;
                        // Check if it's a char literal like 'a' or a lifetime like 'a
                        let word_start = i;
                        while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                            i += 1;
                        }
                        if i < len && bytes[i] == b'\'' {
                            // It's a char literal: 'x'
                            i += 1;
                            self.spans.push(Span {
                                range: start..i,
                                kind: TokenKind::String,
                            });
                        } else {
                            // It's a lifetime: 'a
                            self.spans.push(Span {
                                range: start..i,
                                kind: TokenKind::Lifetime,
                            });
                        }
                        continue;
                    }
                    // Char literal with escape: '\n'
                    if i + 1 < len && bytes[i + 1] == b'\\' {
                        let start = i;
                        i += 2; // skip '\
                        while i < len && bytes[i] != b'\'' {
                            i += 1;
                        }
                        if i < len { i += 1; }
                        self.spans.push(Span {
                            range: start..i,
                            kind: TokenKind::String,
                        });
                        continue;
                    }
                }
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                    } else if bytes[i] == b'\'' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::String,
                });
                continue;
            }

            // Backtick strings (JS template literals, Go raw strings, shell)
            if self.config.has_backtick_strings && bytes[i] == b'`' {
                let start = i;
                i += 1;
                while i < len && bytes[i] != b'`' {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < len { i += 1; }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::String,
                });
                continue;
            }

            // Numbers
            if bytes[i].is_ascii_digit()
                || (bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit())
            {
                // Don't treat as number if preceded by letter/underscore (part of identifier)
                if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
                    // Part of an identifier, treat as identifier continuation
                    let start = i;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    self.spans.push(Span {
                        range: start..i,
                        kind: TokenKind::Default,
                    });
                    continue;
                }
                let start = i;
                // Hex: 0x..., Octal: 0o..., Binary: 0b...
                if bytes[i] == b'0' && i + 1 < len {
                    match bytes[i + 1] {
                        b'x' | b'X' => {
                            i += 2;
                            while i < len && (bytes[i].is_ascii_hexdigit() || bytes[i] == b'_') {
                                i += 1;
                            }
                        }
                        b'o' | b'O' => {
                            i += 2;
                            while i < len && ((bytes[i] >= b'0' && bytes[i] <= b'7') || bytes[i] == b'_') {
                                i += 1;
                            }
                        }
                        b'b' | b'B' => {
                            i += 2;
                            while i < len && (bytes[i] == b'0' || bytes[i] == b'1' || bytes[i] == b'_') {
                                i += 1;
                            }
                        }
                        _ => {
                            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'_' || bytes[i] == b'e' || bytes[i] == b'E') {
                                if (bytes[i] == b'e' || bytes[i] == b'E') && i + 1 < len && (bytes[i + 1] == b'+' || bytes[i + 1] == b'-') {
                                    i += 1;
                                }
                                i += 1;
                            }
                        }
                    }
                } else {
                    while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'_' || bytes[i] == b'e' || bytes[i] == b'E') {
                        if (bytes[i] == b'e' || bytes[i] == b'E') && i + 1 < len && (bytes[i + 1] == b'+' || bytes[i + 1] == b'-') {
                            i += 1;
                        }
                        i += 1;
                    }
                }
                // Rust numeric suffixes: u8, i32, f64, etc.
                if self.language == "rust" && i < len && (bytes[i] == b'u' || bytes[i] == b'i' || bytes[i] == b'f') {
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Dollar variables (PHP, Shell)
            if self.config.has_dollar_vars && bytes[i] == b'$' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Variable,
                });
                continue;
            }

            // Rust macros: name!
            if self.config.has_macros && bytes[i].is_ascii_alphabetic() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                if i < len && bytes[i] == b'!' {
                    i += 1; // include the !
                    self.spans.push(Span {
                        range: start..i,
                        kind: TokenKind::Macro,
                    });
                    continue;
                }
                // Not a macro, process as identifier below
                i = start;
            }

            // Identifiers and keywords
            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];

                // Check what kind of token this is
                let kind = if self.config.constants.contains(&word) {
                    if word == "true" || word == "false" || word == "True" || word == "False" {
                        TokenKind::Boolean
                    } else {
                        TokenKind::Constant
                    }
                } else if self.config.control.contains(&word) {
                    TokenKind::Control
                } else if self.config.keywords.contains(&word) {
                    TokenKind::Keyword
                } else if self.config.types.contains(&word) {
                    TokenKind::Type
                } else if self.config.builtins.contains(&word) {
                    TokenKind::Function
                } else if i < len && (bytes[i] == b'(' || (bytes[i] == b':' && i + 1 < len && bytes[i + 1] == b':')) {
                    // Followed by ( means function call
                    TokenKind::Function
                } else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) && word.len() > 1 {
                    // PascalCase = likely a type (heuristic)
                    TokenKind::Type
                } else if word.chars().all(|c| c.is_uppercase() || c == '_') && word.len() > 1 {
                    // ALL_CAPS = likely a constant
                    TokenKind::Constant
                } else {
                    TokenKind::Default
                };

                self.spans.push(Span {
                    range: start..i,
                    kind,
                });
                continue;
            }

            // Operators
            if matches!(bytes[i], b'=' | b'!' | b'<' | b'>' | b'+' | b'-' | b'*' | b'/' | b'%' | b'&' | b'|' | b'^' | b'~' | b'?') {
                let start = i;
                // Multi-char operators: ==, !=, <=, >=, &&, ||, <<, >>, ->, =>, ::, ..
                i += 1;
                if i < len && matches!(bytes[i], b'=' | b'>' | b'<' | b'&' | b'|' | b'.') {
                    i += 1;
                    // Triple: <<=, >>=, ...
                    if i < len && bytes[i] == b'=' {
                        i += 1;
                    }
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Operator,
                });
                continue;
            }

            // Punctuation
            if matches!(bytes[i], b'(' | b')' | b'{' | b'}' | b'[' | b']' | b';' | b',' | b'.' | b':') {
                self.spans.push(Span {
                    range: i..i + 1,
                    kind: TokenKind::Punctuation,
                });
                i += 1;
                continue;
            }

            // CSS/HTML-specific: # for colors or selectors
            if (self.language == "css" || self.language == "scss" || self.language == "html") && bytes[i] == b'#' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
                    i += 1;
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Constant,
                });
                continue;
            }

            // HTML/XML tags
            if (self.language == "html" || self.language == "htm" || self.language == "xml" || self.language == "svg") && bytes[i] == b'<' {
                let start = i;
                i += 1;
                if i < len && bytes[i] == b'/' { i += 1; }
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
                    i += 1;
                }
                self.spans.push(Span {
                    range: start..i,
                    kind: TokenKind::Tag,
                });
                continue;
            }

            // Default: advance one byte
            self.spans.push(Span {
                range: i..i + 1,
                kind: TokenKind::Default,
            });
            i += 1;
        }
    }
}

/// Detects language from file extension.
pub fn detect_language(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "py" | "pyw" | "pyi" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "typescript",
        "json" | "jsonc" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" | "mdx" => "markdown",
        "html" | "htm" => "html",
        "xml" | "xsl" | "xslt" => "xml",
        "svg" => "svg",
        "css" => "css",
        "scss" => "scss",
        "sass" => "sass",
        "less" => "css",
        "sh" | "bash" => "bash",
        "zsh" => "zsh",
        "fish" => "shell",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "c" | "h" => "c",
        "cpp" | "cxx" | "cc" | "c++" | "hpp" | "hxx" | "hh" | "h++" => "cpp",
        "cs" => "csharp",
        "rb" | "rake" | "gemspec" => "ruby",
        "php" | "phtml" => "php",
        "lua" => "lua",
        "sql" => "sql",
        "dart" => "dart",
        "r" | "R" => "r",
        "pl" | "pm" => "perl",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "hs" | "lhs" => "haskell",
        "ml" | "mli" => "ocaml",
        "scala" | "sc" => "scala",
        "clj" | "cljs" | "cljc" => "clojure",
        "elm" => "elm",
        "vim" => "vim",
        "tf" | "tfvars" => "terraform",
        "dockerfile" | "Dockerfile" => "dockerfile",
        "makefile" | "Makefile" | "mk" => "makefile",
        "cmake" | "CMakeLists.txt" => "cmake",
        "gradle" | "gradle.kts" => "kotlin",
        "graphql" | "gql" => "graphql",
        "proto" => "protobuf",
        "ini" | "cfg" | "conf" => "toml",
        "env" => "shell",
        "gitignore" | "dockerignore" => "shell",
        "txt" | "text" | "log" => "text",
        _ => "text",
    }
}
