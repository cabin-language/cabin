use std::{collections::VecDeque, io::Write as _};

use crate::{
	Context,
	Span,
	SpanLike as _,
	Spanned as _,
	ast::expressions::object::Object,
	comptime::storage::{Evaluated, Pointer},
	diagnostics::DiagnosticInfo,
	log,
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
pub fn call_builtin_at_compile_time(name: &str, context: &mut Context, mut arguments: VecDeque<Pointer<Evaluated>>, span: &Span) -> Pointer<Evaluated> {
	match name {
		"system.terminal.write" => {
			let Some(pointer) = arguments.pop_front() else {
				context.add_diagnostic_at(span, DiagnosticInfo::IncorrectArgumentCount { expected: 1, actual: 0 });

				return context.none();
			};

			// convert the object to a string with the builtin to_text()
			let string_value = call_builtin_at_compile_time("Any.to_text", context, vec![pointer].into(), span).as_string(context).unwrap();

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
			let left = arguments.pop_front().unwrap().as_string(context).unwrap();
			let right = arguments.pop_front().unwrap().as_string(context).unwrap();
			let concatenated = format!("{left}{right}");

			Object::text(concatenated, context, span.to_owned()).store(context).into()
		},

		"terminal.input" => {
			// read a line from stdin
			let mut line = String::new();
			let _ = std::io::stdin().read_line(&mut line).unwrap();

			// trim newline
			line = line.get(0..line.len() - 1).unwrap().to_owned();

			// return
			Object::text(line, context, span.to_owned()).store(context).into()
		},

		"Any.to_text" => {
			let this = arguments.pop_front().unwrap();
			let text = this.value(context).as_object(context).unwrap().to_text(context);

			Object::text(text, context, this.span().owned()).store(context).into()
		},

		"List.insert" => {
			log!(context, "Calling builtin \"List.insert\"");
			let index = arguments.get(1).unwrap().as_number(context).unwrap() as usize;
			let element = arguments.get(2).unwrap().to_owned();
			let this = arguments.pop_front().unwrap().as_mut_list(context).unwrap();
			this.insert(index, element);

			context.none()
		},

		"List.length" => {
			log!(context, "Calling builtin \"List.length\"");
			let this = arguments.pop_front().unwrap().as_list(context).unwrap();
			Object::number(this.len() as f64, context, span.to_owned()).store(context)
		},

		_ => unreachable!(),
	}
}
