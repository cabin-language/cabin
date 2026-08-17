use std::{collections::VecDeque, io::Write as _};

use crate::{
	Context,
	Span,
	ast::expressions::{
		Expression,
		list::List,
		number::Number,
		string::{LiteralPart, Text},
	},
	comptime::storage::Pointer,
	diagnostics::DiagnosticInfo,
	lexer::{Token, TokenType},
	log,
	traits::TryAs,
};

/// Calls a built-in function at compile-time. Built-in functions are called at compiled time with Rust code. This is used in
/// `FunctionCall::evaluate_at_compile_time()` to evaluate function-call expressions at compile-time when the function to call is a built-in function.
///
/// # Parameters
///
/// - `name` - The name of the built-in functions to call
/// - `arguments` - The arguments passed to the built-in function.
/// - `span` - The span of the entire function call, including the callee and the arguments.
///
/// # Returns
///
/// The return value from the built-in function; Possibly `Expression::Void`.
///
/// # Errors
///
/// If there is no built-in function with the given name, an error is returned.
///
/// Also, if the built-in function throws an error while being called, that error is returned as well.
pub fn call_builtin_at_compile_time(name: &str, context: &mut Context, mut arguments: VecDeque<Pointer>, span: &Span) -> Pointer {
	match name {
		"system.terminal.write" => {
			let Some(pointer) = arguments.pop_front() else {
				context.add_diagnostic_at(span, DiagnosticInfo::IncorrectArgumentCount {
					expected: 1,
					actual: 0,
					name: "builtin::system::terminal::write".to_owned(),
				});

				return context.none();
			};

			// convert the object to a string with the builtin to_text()
			let string_value = call_builtin_at_compile_time("Any.to_text", context, vec![pointer].into(), span)
				.as_text_literal(context)
				.unwrap()
				.to_owned();

			if context.side_effects {
				if !context.has_printed && !context.interactive {
					context.has_printed = true;
					println!();
				}
				print!("{string_value}");
				std::io::stdout().flush().unwrap();
			}

			// Add hint diagnostic
			if !context.interactive {
				context.add_diagnostic_at(span, DiagnosticInfo::Info(string_value.trim().to_owned()));
			}

			context.none()
		},

		"Text.plus" => {
			// safe to unwrap here because these will be typechecked before the call ever happens
			// if the arguments aren't present or aren't strings this won't get called
			let left = arguments.pop_front().unwrap().as_text_literal(context).unwrap();
			let right = arguments.pop_front().unwrap().as_text_literal(context).unwrap();
			let concatenated = format!("{left}{right}");

			Expression::from(Text::literal(
				LiteralPart {
					value: concatenated,
					span: span.to_owned(),
				},
				context,
			))
			.store(context)
		},

		"terminal.input" => {
			// read a line from stdin
			let mut line = String::new();
			let _ = std::io::stdin().read_line(&mut line).unwrap();

			// trim newline
			line = line.get(0..line.len() - 1).unwrap().to_owned();

			// return
			Expression::from(Text::literal(
				LiteralPart {
					value: line,
					span: span.to_owned(),
				},
				context,
			))
			.store(context)
		},

		"Any.to_text" => {
			let this = arguments.pop_front().unwrap();
			let text = this.value(context).as_object().unwrap().to_text(context);

			Expression::from(Text::literal(
				LiteralPart {
					value: text,
					span: span.to_owned(),
				},
				context,
			))
			.store(context)
		},

		"List.insert" => {
			log!(context, "Calling builtin \"List.insert\"");

			if arguments.len() != 3 {
				context.add_diagnostic_at(span, DiagnosticInfo::IncorrectArgumentCount {
					expected: 1,
					actual: 0,
					name: "builtin::List::insert".to_owned(),
				});

				return context.none();
			}

			dbg!(arguments[1].value(context));
			let this = arguments.get(0).unwrap().as_mut_list_literal(context).unwrap();
			let index = arguments.get(1).unwrap().as_number_literal(context).unwrap() as usize;
			let element = arguments.get(2).unwrap().to_owned();
			// this.insert(index, element, context);

			context.none()
		},

		"List.length" => {
			log!(context, "Calling builtin \"List.length\"");
			let this = arguments.pop_front().unwrap().as_list_literal(context).unwrap();
			Expression::Number(Number::synthetic(
				Token::synthetic(TokenType::Number, this.elements().len().to_string(), span.to_owned()),
				context,
			))
			.store(context)
		},

		_ => unreachable!(),
	}
}
