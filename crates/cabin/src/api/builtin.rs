use std::{collections::VecDeque, fmt::Write as _, io::Write};

use super::context::Context;
use crate::{
	api::{macros::string, scope::ScopeId, traits::TryAs as _},
	comptime::{memory::VirtualPointer, CompileTime},
	lexer::Span,
	parser::expressions::{name::Name, object::ObjectConstructor, Expression},
};

pub struct BuiltinFunction {
	evaluate_at_compile_time: fn(&mut Context, ScopeId, Vec<Expression>, Span) -> Expression,
}

static BUILTINS: phf::Map<&str, BuiltinFunction> = phf::phf_map! {
	"terminal.print" => BuiltinFunction {
		evaluate_at_compile_time: |context, caller_scope_id, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let pointer = arguments.pop_front().unwrap_or_else(|| Expression::ErrorExpression(span));
			let returned_object = call_builtin_at_compile_time("Anything.to_string", context, caller_scope_id, vec![pointer], span);
			let string_value = returned_object.try_as_literal(context).try_as::<String>().unwrap().to_owned();

			if context.lines_printed == 0 && !context.config().options().quiet() {
				//println!("\n");
				context.lines_printed += 1;
			}

			println!("{string_value}");
			context.lines_printed += string_value.chars().filter(|character| character == &'\n').count() + 1;

			Expression::ErrorExpression(Span::unknown())
		},
	},
	"terminal.input" => BuiltinFunction {
		evaluate_at_compile_time: |context, _caller_scope_id, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let options = arguments.pop_front().unwrap().try_as::<VirtualPointer>().unwrap_or(&VirtualPointer::ERROR).virtual_deref(context);
			let prompt = options.get_field_literal("prompt").unwrap().virtual_deref(context).get_internal_field::<String>("internal_value").unwrap().to_owned();

			if context.lines_printed == 0 {
				println!("\n");
				context.lines_printed += 1;
			}

			context.lines_printed += 1;
			print!("{prompt}");
			std::io::stdout().flush().unwrap();
			let mut line = String::new();
			let _ = std::io::stdin().read_line(&mut line).unwrap();
			line = line.get(0..line.len() - 1).unwrap().to_owned();
			Expression::Pointer(*ObjectConstructor::string(&line, span, context).evaluate_at_compile_time(context).try_as::<VirtualPointer>().unwrap_or(&VirtualPointer::ERROR))
		},
	},
	"Anything.to_string" => BuiltinFunction {
		evaluate_at_compile_time: |context, _caller_scope_id, arguments,span| {
			let this = arguments
				.first()
				.unwrap_or(&Expression::ErrorExpression(span))
				.try_as_literal(context);

			let type_name = this.get_internal_field::<Name>("representing_type_name").unwrap_or_else(|_| this.type_name());
			string(&match type_name.unmangled_name() {
				"Number" => this.try_as::<f64>().unwrap().to_string(),
				"Text" => this.try_as::<String>().unwrap().to_owned(),
				_ => {
					let mut builder = "{".to_owned();

					for (field_name, field_pointer) in this.fields() {
						write!(builder, "\n\t{} = {field_pointer},", field_name.unmangled_name()).unwrap();
					}

					if !this.has_any_fields() {
						builder += "\n";
					}

					builder += "}";

					builder
				}
			}, span, context)
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
pub fn call_builtin_at_compile_time(name: &str, context: &mut Context, caller_scope_id: ScopeId, arguments: Vec<Expression>, span: Span) -> Expression {
	(BUILTINS.get(name).unwrap().evaluate_at_compile_time)(context, caller_scope_id, arguments, span)
}
