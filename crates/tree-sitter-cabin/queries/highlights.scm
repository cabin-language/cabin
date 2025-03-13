; Keywords
(declaration ["let"] @keyword)
(if_expression ["if" "otherwise"] @keyword)
(group ["group"] @keyword)
(either ["either"] @keyword)
(function ["action"] @keyword.function)
(object_constructor ["new"] @keyword)
(extend ["tobe" "extensionof"] @keyword)
(goto ["is"] @keyword)
(foreach ["foreach" "in"] @keyword)
(while_loop ["while"] @keyword)
(binary ["and" "or"] @keyword)
(match ["match" "otherwise"] @keyword)
(function_call ["let"] @keyword)

; Semantics
(type (expression (literal (identifier))) @type)
(goto label: (identifier) @label)
(group_parameter name: (identifier) @type)
(function_call callee: (expression (literal (identifier))) @function.call)
(function_call
	callee: (expression (binary operator: "." right: (identifier) @function.call))
	(#set! "priority" 110)
)
(binary operator: "." right: (identifier) @variable.member)
(parameter name: (identifier) @variable.parameter)
(group_field name: (identifier) @variable.member)
(group_field
	name: (identifier) @function
	value: (expression (literal (function)))
)
(group_field
	name: (identifier) @function
	type: (type (expression (literal (function))))
)
(object_constructor type: (identifier) @type)
(object_value name: (identifier) @variable.member)
(object_value
	name: (identifier) @function
	value: (expression (literal (function)))
)
(declaration
	name: (identifier) @type
	value: (expression (literal (group)))
)
(declaration
	name: (identifier) @type
	value: (expression (literal (either)))
)
(declaration
	name: (identifier) @type
	value: (expression (literal (extend)))
)
(declaration
	name: (identifier) @function
	value: (expression (literal (function)))
)
(either_variant name: (identifier) @lsp.type.enumMember)

(expression (literal (identifier (other_identifier))) @boolean (#any-of? @boolean "true" "false"))
(expression (literal (identifier (other_identifier))) @constant.builtin (#any-of? @constant.builtin "nothing"))
(expression (literal (identifier (other_identifier))) @variable (#not-any-of? @variable "true" "false" "nothing"))

; Brackets
["(" ")" "[" "]" "{" "}" "<" ">"] @punctuation.bracket
[";" ":" "," "." "=>"] @punctuation.delimiter
["+" "-" "*" "/" "^" "==" "!=" "<=" ">=" "< " " >" "="] @operator
(tag ["#"] @punctuation.special)

; Tokens
(number) @number
(string) @string
(comment) @comment
(pascal_case_identifier) @type
