use std::collections::VecDeque;

use crate::{
	api::traits::TryAs as _,
	ast::{
		expressions::{new_literal::EvaluatedLiteral, Expression},
		sugar::string::CabinString,
	},
	comptime::memory::ExpressionPointer,
	diagnostics::{Diagnostic, DiagnosticInfo},
	Context,
	Span,
	Spanned,
};

pub struct BuiltinFunction {
	evaluate_at_compile_time: fn(&mut Context, Vec<ExpressionPointer>, Span) -> ExpressionPointer,
}

static BUILTINS: phf::Map<&str, BuiltinFunction> = phf::phf_map! {
	"terminal.print" => BuiltinFunction {
		evaluate_at_compile_time: |context, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let pointer = arguments.pop_front().unwrap_or_else(|| Expression::error(span, context));
			let returned_object = call_builtin_at_compile_time("Anything.to_string", context, vec![pointer], span);
			let string_value = returned_object.as_literal(context).evaluated_literal(context).try_as::<CabinString>().unwrap().value.to_owned();

			if context.side_effects {
				if !context.has_printed {
					context.has_printed = true;
					println!();
				}
				println!("{string_value}");
			}

			// Add hint diagnostic
			if pointer != ExpressionPointer::ERROR {
				context.add_diagnostic(Diagnostic {
					span: pointer.span(context),
					info: DiagnosticInfo::Info(string_value),
					file: context.file.clone()
				});
			}

			Expression::error(Span::unknown(), context)
		},
	},
	"terminal.debug" => BuiltinFunction {
		evaluate_at_compile_time: |context, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let pointer = arguments.pop_front().unwrap_or_else(|| Expression::error(span, context));
			let returned_object = call_builtin_at_compile_time("Anything.to_string", context, vec![pointer], span);
			let string_value = returned_object.as_literal(context).evaluated_literal(context).try_as::<CabinString>().unwrap().value.to_owned();

			if context.side_effects {
				if !context.has_printed {
					context.has_printed = true;
					println!();
				}
				println!("{string_value}");
			}

			// Add hint diagnostic
			if pointer != ExpressionPointer::ERROR {
				context.add_diagnostic(Diagnostic {
					span: pointer.span(context),
					info: DiagnosticInfo::Info(string_value),
					file: context.file.clone()
				});
			}

			Expression::error(Span::unknown(), context)
		},
	},
	"Text.plus" => BuiltinFunction {
		evaluate_at_compile_time: |context, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let this = arguments.pop_front().unwrap_or_else(|| Expression::error(span, context));
			let other = arguments.pop_front().unwrap_or_else(|| Expression::error(span, context));

			let EvaluatedLiteral::String(string) = this.as_literal(context).evaluated_literal(context).to_owned() else { unreachable!() };
			let EvaluatedLiteral::String(string2) = other.as_literal(context).evaluated_literal(context) else { unreachable!() };

			Expression::EvaluatedLiteral(EvaluatedLiteral::String(CabinString { value: string.value + &string2.value, span })).store_in_memory(context)
		},
	},
	"terminal.input" => BuiltinFunction {
		evaluate_at_compile_time: |context, _arguments, _span| {
			let mut line = String::new();
			let _ = std::io::stdin().read_line(&mut line).unwrap();
			line = line.get(0..line.len() - 1).unwrap().to_owned();
			Expression::EvaluatedLiteral(EvaluatedLiteral::String(CabinString { value: line, span: Span::unknown() })).store_in_memory(context)
		},
	},

	"Anything.to_string" => BuiltinFunction {
		evaluate_at_compile_time: |context, arguments, span| {
			let this = arguments.first().unwrap_or(&Expression::error(span, context)).as_literal(context).evaluated_literal(context);

			Expression::EvaluatedLiteral(EvaluatedLiteral::String(CabinString { span: Span::unknown(), value: match this {
				EvaluatedLiteral::Number(number) => number.to_string(),
				EvaluatedLiteral::String(string) => string.value.clone(),
				_ => "<object>".to_owned()
			}}))
			.store_in_memory(context)
		},
	},
};

/// Calls a built-in function at compile-time. Built-in functions are called at compiled time with Rust code. This is used in
/// `FunctionCall::evaluate_at_compile_time()` to evaluate function-call expressions at compile-time when the function to call is a built-in function.
///
/// # Parameters
///
/// - `name` - The name of the built-in functions to call; This must be a key in the `BUILTINS` map.
/// - `caller_scope_id` - The scope id of the site at which the function was called.
/// - `arguments` - The arguments passed to the built-in function.
/// - `span` - The span of the function call.
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
pub fn call_builtin_at_compile_time(name: &str, context: &mut Context, arguments: Vec<ExpressionPointer>, span: Span) -> ExpressionPointer {
	(BUILTINS.get(name).unwrap().evaluate_at_compile_time)(context, arguments, span)
}
