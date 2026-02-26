; TOML syntax highlighting queries

; Section headers
(table (bare_key) @namespace)
(table (dotted_key) @namespace)

; Keys
(pair (bare_key) @property)
(pair (dotted_key) @property)

; String values
(string) @string

; Numbers
(integer) @number
(float) @number

; Booleans
(boolean) @constant.builtin

; Dates
(local_date) @constant
(local_time) @constant
(local_date_time) @constant
(offset_date_time) @constant

; Comments
(comment) @comment

; Punctuation
[
  "["
  "]"
  "[["
  "]]"
  "{"
  "}"
  ","
  "."
  "="
] @punctuation
