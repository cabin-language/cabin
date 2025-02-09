use std::{collections::VecDeque, fmt::Write as _, io::Write};

use colored::Colorize as _;

use crate::{
	api::{context::context, macros::string, scope::ScopeId, traits::TryAs as _},
	comptime::{memory::VirtualPointer, CompileTime},
	err,
	lexer::Span,
	parser::expressions::{name::Name, object::ObjectConstructor, Expression},
};

pub struct BuiltinFunction {
	evaluate_at_compile_time: fn(ScopeId, Vec<Expression>, Span) -> Expression,
	to_c: fn(&[String]) -> anyhow::Result<String>,
}

static BUILTINS: phf::Map<&str, BuiltinFunction> = phf::phf_map! {
	"terminal.print" => BuiltinFunction {
		evaluate_at_compile_time: |caller_scope_id, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let pointer = arguments.pop_front().unwrap_or_else(|| Expression::ErrorExpression(span));
			let returned_object = call_builtin_at_compile_time("Anything.to_string", caller_scope_id, vec![pointer], span);
			let string_value = returned_object.try_as_literal().unwrap().try_as::<String>().unwrap().to_owned();

			let options = arguments.pop_front().unwrap().try_as::<VirtualPointer>().unwrap_or(&VirtualPointer::ERROR).virtual_deref();
			let newline = Expression::Pointer(options.get_field("newline").unwrap()).is_true();

			if context().lines_printed == 0 && !context().config().options().quiet() {
				println!("\n");
				context().lines_printed += 1;
			}

			if newline {
				println!("{string_value}");
			} else {
				print!("{string_value}");
				std::io::stdout().flush().unwrap();
			}
			context().lines_printed += string_value.chars().filter(|character| character == &'\n').count() + 1;

			Expression::ErrorExpression(Span::unknown())
		},
		to_c: |parameter_names| {
			let text_address = context().scope_data.get_variable("Text").unwrap().try_as::<VirtualPointer>().unwrap();
			let anything_address = context().scope_data.get_variable("Anything").unwrap().try_as::<VirtualPointer>().unwrap();
			let object = parameter_names.first().unwrap();
			Ok(unindent::unindent(&format!(
				"
				group_u_Text_{text_address}* return_address;
				(((void (*)(group_u_Anything_{anything_address}*, group_u_Text_{text_address}*))({object}->u_to_string->call))({object}, return_address));
				printf(\"%s\\n\", return_address->internal_value);
				"
			)))
		}
	},
	"terminal.input" => BuiltinFunction {
		evaluate_at_compile_time: |_caller_scope_id, arguments, span| {
			let mut arguments = VecDeque::from(arguments);
			let options = arguments.pop_front().unwrap().try_as::<VirtualPointer>().unwrap_or(&VirtualPointer::ERROR).virtual_deref();
			let prompt = options.get_field_literal("prompt").unwrap().get_internal_field::<String>("internal_value").unwrap();

			if context().lines_printed == 0 {
				println!("\n");
				context().lines_printed += 1;
			}

			context().lines_printed += 1;
			print!("{prompt}");
			std::io::stdout().flush().unwrap();
			let mut line = String::new();
			let _ = std::io::stdin().read_line(&mut line).unwrap();
			line = line.get(0..line.len() - 1).unwrap().to_owned();
			Expression::Pointer(*ObjectConstructor::string(&line, span).evaluate_at_compile_time().try_as::<VirtualPointer>().unwrap_or(&VirtualPointer::ERROR))
		},
		to_c: |parameter_names| {
			let return_address = parameter_names.first().unwrap();
			let text_address = context().scope_data.get_variable("Text").unwrap().try_as::<VirtualPointer>().unwrap();
			Ok(format!("char* buffer = malloc(sizeof(char) * 256);\nfgets(buffer, 256, stdin);\n*{return_address} = (group_u_Text_{text_address}) {{ .internal_value = buffer }};"))
		}
	},
	"Anything.to_string" => BuiltinFunction {
		evaluate_at_compile_time: |_caller_scope_id, arguments,span| {
			let this = arguments
				.first()
				.unwrap_or(&Expression::ErrorExpression(span))
				.try_as_literal().unwrap();

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
			}, span)
		},
		to_c: |parameter_names| {
			let object = parameter_names.first().unwrap();
			let return_address = parameter_names.get(1).ok_or_else(|| err! {
				base = format!("Missing first argument to {}", format!("{}.{}()", "Anything".yellow(), "to_string".blue()).bold()),
				while = format!("getting the first argument to {}", format!("{}.{}()", "Anything".yellow(), "to_string".blue()).bold()),
			})?;

			let group_address = context().scope_data.get_variable("Group").unwrap().try_as::<VirtualPointer>().unwrap();
			let text_address = context().scope_data.get_variable("Text").unwrap().try_as::<VirtualPointer>().unwrap();
			let anything_address = context().scope_data.get_variable("Anything").unwrap().try_as::<VirtualPointer>().unwrap();

			Ok(unindent::unindent(&format!(
				r#"
				// Get the type metadata of the value
				group_u_Group_{group_address} type;
				(((void (*)(group_u_Anything_{anything_address}*, group_u_Group_{group_address}*))({object}->u_type->call))({object}, &type));

				// Build the string
				DynamicString result = (DynamicString) {{ .value = type.name, .capacity = 16 }};
				push_to_dynamic_string(&result, " {{");

				// Add fields
				for (int fieldNumber = 0; fieldNumber < type.u_fields->elements.size; fieldNumber++) {{										
					push_to_dynamic_string(&result, (char*) type.u_fields->elements.data[fieldNumber]);
				}}

				// Return the built string
				push_to_dynamic_string(&result, " }}");
				*{return_address} = (group_u_Text_{text_address}) {{ .internal_value = result.value }};
				"#
			)))
		}
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
pub fn call_builtin_at_compile_time(name: &str, caller_scope_id: ScopeId, arguments: Vec<Expression>, span: Span) -> Expression {
	(BUILTINS.get(name).unwrap().evaluate_at_compile_time)(caller_scope_id, arguments, span)
}

/// Transpiles a built-in function to C code. This is used during transpilation to supply built-in functions with bodies.
///
/// # Parameters
///
/// - `name` - The name of the built-in functions to call; This must be a key in the `BUILTINS` map.
/// - `parameter_names` - The names of the parameters of the function to transpile.
///
/// # Returns
///
/// The C code after transpilation.
///
/// # Errors
///
/// If there is no built-in function with the given name, an error is returned.
///
/// Also, if the built-in function throws an error while being transpiled, that error is returned as well.
pub fn transpile_builtin_to_c(name: &str, parameters: &[String]) -> anyhow::Result<String> {
	(BUILTINS
		.get(name)
		.ok_or_else(|| {
			anyhow::anyhow!(
				"Attempted to transpile the built-in function \"{}\" to C, but no built-in function with that name exists.",
				name.bold().cyan()
			)
		})?
		.to_c)(parameters)
}
