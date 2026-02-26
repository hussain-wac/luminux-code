; Python syntax highlighting queries

; Keywords
[
  "and"
  "as"
  "assert"
  "async"
  "await"
  "break"
  "class"
  "continue"
  "def"
  "del"
  "elif"
  "else"
  "except"
  "finally"
  "for"
  "from"
  "global"
  "if"
  "import"
  "in"
  "is"
  "lambda"
  "nonlocal"
  "not"
  "or"
  "pass"
  "raise"
  "return"
  "try"
  "while"
  "with"
  "yield"
] @keyword

; Functions
(function_definition name: (identifier) @function)
(call function: (identifier) @function.call)
(call function: (attribute attribute: (identifier) @function.method))

; Classes
(class_definition name: (identifier) @type)

; Variables
(identifier) @variable
(attribute attribute: (identifier) @property)

; Parameters
(parameters (identifier) @parameter)

; Literals
(string) @string
(integer) @number
(float) @number
(true) @constant.builtin
(false) @constant.builtin
(none) @constant.builtin

; Comments
(comment) @comment

; Decorators
(decorator) @attribute

; Operators
[
  "+"
  "-"
  "*"
  "/"
  "//"
  "%"
  "**"
  "="
  "+="
  "-="
  "*="
  "/="
  "//="
  "%="
  "**="
  "=="
  "!="
  "<"
  ">"
  "<="
  ">="
  "@"
  "@="
  "&"
  "|"
  "^"
  "~"
  "<<"
  ">>"
  "&="
  "|="
  "^="
  "<<="
  ">>="
] @operator

; Punctuation
[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  ","
  ":"
  "."
] @punctuation
