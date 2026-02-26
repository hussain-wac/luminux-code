; JSON syntax highlighting queries

; Keys (property names)
(pair key: (string) @property)

; String values
(string) @string

; Numbers
(number) @number

; Booleans and null
(true) @constant.builtin
(false) @constant.builtin
(null) @constant.builtin

; Punctuation
[
  "{"
  "}"
  "["
  "]"
  ","
  ":"
] @punctuation
