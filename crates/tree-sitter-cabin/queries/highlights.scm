; Keywords
(declaration ["let"] @keyword)
(group ["group"] @keyword)
(either ["either"] @keyword)
(function ["action"] @keyword.function)
(object_constructor ["new"] @keyword)
(extend ["tobe" "extensionof"] @keyword)
(goto ["is" "done"] @keyword)
(foreach ["foreach" "in"] @keyword)

; Semantics
(compile_time_argument (expression (literal (identifier))) @type)
(goto label: (identifier) @label)
(group_parameter name: (identifier) @type)
(group_parameter type: (expression (literal (identifier))) @type)
(extend target: (expression (literal (identifier))) @type)
(extend tobe: (expression (literal (identifier))) @type )
(function return_type: (expression (literal (identifier))) @type)
(function_call callee: (expression (literal (identifier))) @function.call)
(function_call
	callee: (expression (binary operator: "." right: (identifier) @function.call))
	(#set! "priority" 110)
)
(binary operator: "." right: (identifier) @variable.member)
(parameter
	name: (identifier) @variable.parameter
	type: (expression (literal (identifier))) @type
)
(group_field name: (identifier) @variable.member)
(group_field type: (expression (literal (identifier))) @type)
(group_field
	name: (identifier) @function
	value: (expression (literal (function)))
)
(group_field
	name: (identifier) @function
	type: (expression (literal (function)))
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
	type: (expression (literal (identifier)) @type)
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

; Brackets
["(" ")" "[" "]" "{" "}" "<" ">"] @punctuation.bracket
[";" ":" "," "."] @punctuation.delimiter
["+" "-" "*" "/" "^" "==" "!=" "<=" ">=" "< " " >" "="] @operator
(tag ["#"] @punctuation.special)

; Tokens
(number) @number
(string) @string
(comment) @comment
(either_variant) @lsp.type.enumMember
