/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
	name: "cabin",

	rules: {
		source_file: $ => repeat($.statement),

		expression: $ => choice(
			$.binary,
			seq("(", $.expression, ")"),
			$.postfix,
			$.literal,
		),

		postfix: $ => prec.left(6, choice(
			$.function_call,
			seq($.expression, "?"),
			seq($.expression, "!"),
		)),

		function_call: $ => prec.left(5, choice(
			seq(
				field("callee", $.expression),
				"<", field("compile_time_arguments", list($.type)), ">",
				optional(seq("(", field("arguments", list(seq(optional("let"), $.expression))), ")"))
			),
			seq(
				field("callee", $.expression),
				seq("(", field("arguments", list(seq(optional("let"), $.expression))), ")")
			),
		)),

		binary: $ => choice(
			prec.left(6, seq(field("left", $.expression), field("operator", choice(".", "::")), field("right", $.identifier))),
			prec.right(5, seq(field("left", $.expression), "^", field("right", $.expression))),
			prec.left(4, seq(field("left", $.expression), choice("*", "/"), field("right", $.expression))),
			prec.left(3, seq(field("left", $.expression), choice("+", "-"), field("right", $.expression))),
			prec.left(2, seq(field("left", $.expression), choice("==", "!=", "<=", ">=", "< ", " >"), field("right", $.expression))),
			prec.left(1, seq(field("left", $.expression), choice("and", "or"), field("right", $.expression)))
		),

		literal: $ => choice(
			$.function,
			$.string,
			$.raw_string,
			$.number,
			$.list,
			$.group,
			$.either,
			$.object_constructor,
			$.extend,
			$.foreach,
			$.if_expression,
			$.while_loop,
			$.match,
			$.block,
			$.run,
			$.identifier,
		),

		run: $ => seq(
			"run",
			"(",
			$.expression,
			")"
		),

		match: $ => seq(
			"match",
			field("source", $.expression),
			"{",
			listWithoutTrailingComma(seq($.expression, "=>", $.expression)),
			optional(seq(",", optional(seq("otherwise", "=>", $.expression, optional(","))))),
			"}"
		),

		if_expression: $ => seq(
			"if",
			field("condition", $.expression),
			field("body", $.block),
			repeat(seq("otherwise", "if", $.expression, $.block)),
			optional(seq("otherwise", $.block)),
		),

		foreach: $ => seq(
			"foreach",
			field("binding", $.identifier),
			"in",
			field("iterable", $.expression),
			$.block
		),

		while_loop: $ => seq(
			"while",
			field("condition", $.expression),
			field("body", $.block),
		),

		object_constructor: $ => seq(
			"new",
			optional(field("type", $.identifier)),
			optional(seq("<", field("compile_time_arguments", list($.type)), ">")),
			"{",
			list($.object_value),
			"}"
		),

		object_value: $ => seq(
			optional(field("tags", $.tag)),
			field("name", $.identifier),
			"=",
			field("value", $.expression)
		),

		extend: $ => seq(
			"extensionof",
			field("target", $.type),
			optional(seq("tobe", field("tobe", $.type))),
			"{",
			list($.object_value),
			"}"
		),

		pattern: $ => seq(
			$.expression
		),

		block: $ => seq("{", repeat($.statement), "}"),
		list: $ => seq("[", list($.expression), "]"),
		tag: $ => seq("#", "[", list($.expression), "]"),

		group: $ => seq(
			"group",
			optional(field("compile_time_parameters", seq("<", list($.group_parameter), ">"))),
			"{",
			field("fields", list($.group_field)),
			"}"
		),

		group_parameter: $ => seq(
			field("name", $.identifier),
			optional(seq(":", field("type", $.type)))
		),

		group_field: $ => seq(
			optional(field("tags", $.tag)),
			field("name", $.identifier),
			optional(seq(":", field("type", $.type))),
			optional(seq("=", field("value", $.expression))),
		),

		either: $ => seq(
			"either",
			optional(seq("<", field("compile_time_parameters", list($.group_parameter)), ">")),
			"{",
			field("variants", list($.either_variant)),
			"}"
		),

		type: $ => $.expression,

		either_variant: $ => seq(field("name", $.identifier), optional(seq(":", field("type", $.type)))),

		function: $ => prec.left(seq(
			"action",
			optional(field("compile_time_parameters", seq("<", list($.group_parameter), ">"))),
			optional(field("parameters", seq("(", list($.parameter), ")"))),
			optional(seq(":", field("return_type", $.type))),
			optional(field("body", $.block))
		)),

		statement: $ => seq(choice(
			$.declaration,
			$.goto,
			$.expression,
		), ";"),

		declaration: $ => seq(
			optional(field("tags", $.tag)),
			"let",
			field("name", $.identifier),
			optional(seq(":", field("type", $.type))),
			optional(seq("=", field("value", $.expression))),
		),

		goto: $ => seq(
			field("label", $.identifier),
			"is",
			field("value", $.expression),
		),

		// Tokens

		pascal_case_identifier: _$ => /[A-Z]\w*/,
		other_identifier: _$ => /[a-z_]\w*/,
		identifier: $ => choice($.pascal_case_identifier, $.other_identifier),

		raw_string: $ => /`[^`]*`/,

		string: $ => seq(
			'"',
			repeat(choice(
				seq("{", $.expression, "}"),
				/[^"\{\}]+/
			)),
			'"',
		),

		number: _$ => /-?\d+(\.\d+)?/,

		// Utilities

		parameter: $ => seq(
			field("name", $.identifier),
			":",
			field("type", $.type),
		),

		// Extra

		comment: _$ => seq('# ', /[^\r\n\u2028\u2029]*/),

	},

	extras: $ => [
		$.comment,
		/[\s\p{Zs}\uFEFF\u2028\u2029\u2060\u200B]/u, // whitespace
	],
});

/**
 * A comma separated, possibly empty list, allowing trailing commas.
 *
 * @param {RuleOrLiteral} rule
 */
function list(rule) {
	return optional(seq(rule, repeat(seq(",", rule)), optional(",")));
}

/**
 * A comma separated, possibly empty list, allowing trailing commas.
 *
 * @param {RuleOrLiteral} rule
 */
function listWithoutTrailingComma(rule) {
	return optional(seq(rule, repeat(seq(",", rule))));
}

