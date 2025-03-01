use std::{collections::VecDeque, io::Write};

use crate::{
	api::{scope::ScopeId, traits::TryAs as _},
	ast::{
		expressions::{
			new_literal::{Literal, Object},
			Expression,
		},
		sugar::string::CabinString,
	},
	comptime::memory::ExpressionPointer,
	Context,
	Span,
};

pub struct BuiltinFunction {
	evaluate_at_compile_time: fn(&mut Context, ScopeId, Vec<ExpressionPointer>, Span) -> ExpressionPointer,
}

static BUILTINS: phf::Map<&str, BuiltinFunction> = phf::phf_map! {
	"terminal.print" => BuiltinFunction {
		evaluate_at_compile_time: |context, caller_scope_id, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let pointer = arguments.pop_front().unwrap_or_else(|| Expression::error(span, context));
			let returned_object = call_builtin_at_compile_time("Anything.to_string", context, caller_scope_id, vec![pointer], span);
			let string_value = returned_object.as_literal(context).literal(context).try_as::<CabinString>().unwrap().value.to_owned();

			println!("{string_value}");

			Expression::error(Span::unknown(), context)
		},
	},
	"terminal.input" => BuiltinFunction {
		evaluate_at_compile_time: |context, _caller_scope_id, arguments, _span| {
			let mut arguments = VecDeque::from(arguments);
			let options = arguments.pop_front().unwrap().as_literal(context).literal(context).try_as::<Object>().unwrap();
			let prompt = options.get_field("prompt").unwrap().literal(context).try_as::<CabinString>().unwrap().value.to_owned();

			print!("{prompt}");
			std::io::stdout().flush().unwrap();
			let mut line = String::new();
			let _ = std::io::stdin().read_line(&mut line).unwrap();
			line = line.get(0..line.len() - 1).unwrap().to_owned();
			Expression::Literal(Literal::String(CabinString { value: line, span: Span::unknown() })).store_in_memory(context)
		},
	},

	"Anything.to_string" => BuiltinFunction {
		evaluate_at_compile_time: |context, _caller_scope_id, arguments,span| {
			let this = arguments.first().unwrap_or(&Expression::error(span, context)).as_literal(context).literal(context);

			Expression::Literal(Literal::String(CabinString { span: Span::unknown(), value: match this {
				Literal::Number(number) => number.to_string(),
				Literal::String(string) => string.value.clone(),
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
pub fn call_builtin_at_compile_time(name: &str, context: &mut Context, caller_scope_id: ScopeId, arguments: Vec<ExpressionPointer>, span: Span) -> ExpressionPointer {
	(BUILTINS.get(name).unwrap().evaluate_at_compile_time)(context, caller_scope_id, arguments, span)
}
