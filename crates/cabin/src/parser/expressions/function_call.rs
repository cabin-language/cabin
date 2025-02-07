use std::{
	collections::{HashMap, VecDeque},
	fmt::Write as _,
};

use colored::Colorize;
use regex_macro::regex;

use crate::{
	api::{
		builtin::call_builtin_at_compile_time,
		context::context,
		scope::{ScopeData, ScopeId},
		traits::TryAs as _,
	},
	bail_err,
	comptime::{memory::VirtualPointer, CompileTime},
	debug_log,
	debug_start,
	if_then_else_default,
	lexer::{Span, Token, TokenType},
	mapped_err,
	parse_list,
	parser::{
		expressions::{
			field_access::{FieldAccess, FieldAccessType},
			function_declaration::FunctionDeclaration,
			literal::{CompilerWarning, LiteralConvertible, LiteralObject},
			name::Name,
			object::{Field, ObjectConstructor},
			run::RuntimeableExpression,
			unary::{UnaryOperation, UnaryOperator},
			Expression,
			Spanned,
			TryParse,
			Typed,
		},
		statements::tag::TagList,
		ListType,
		Parse as _,
		TokenQueueFunctionality,
	},
	transpiler::TranspileToC,
	warn,
};

#[derive(Debug, Clone)]
enum Argument {
	Positional(Expression),
	Keyword(Name, Expression),
}

fn composite_arguments(arguments: Vec<Argument>) -> Vec<Expression> {
	let mut output = Vec::new();
	let mut keyword_arguments = Vec::new();
	let mut has_keyword_arguments = false;
	for argument in arguments {
		match argument {
			Argument::Positional(value) => output.push(value),
			Argument::Keyword(name, value) => {
				has_keyword_arguments = true;
				keyword_arguments.push(Field {
					name,
					value: Some(value),
					field_type: None,
				});
			},
		}
	}
	let composite_keyword_argument = Expression::ObjectConstructor(ObjectConstructor {
		fields: keyword_arguments,
		type_name: "Object".into(),
		internal_fields: HashMap::new(),
		outer_scope_id: context().scope_data.unique_id(),
		inner_scope_id: context().scope_data.unique_id(),
		field_access_type: FieldAccessType::Normal,
		name: "options".into(),
		span: Span::unknown(),
		tags: TagList::default(),
	});

	if has_keyword_arguments {
		output.push(composite_keyword_argument);
	}

	output
}

#[derive(Debug, Clone)]
pub struct FunctionCall {
	function: Box<Expression>,
	compile_time_arguments: Vec<Expression>,
	arguments: Vec<Expression>,
	scope_id: ScopeId,
	span: Span,
	pub tags: TagList,
	has_keyword_arguments: bool,
	has_keyword_compile_time_arguments: bool,
}

pub struct PostfixOperators;

impl TryParse for PostfixOperators {
	type Output = Expression;

	fn try_parse(tokens: &mut VecDeque<Token>) -> Result<Self::Output, crate::Diagnostic> {
		// Primary expression
		let mut expression = FieldAccess::try_parse(tokens)?;
		let start = expression.span();
		let mut end = start;

		// Postfix function call operators
		while tokens.next_is_one_of(&[
			TokenType::LeftParenthesis,
			TokenType::LeftAngleBracket,
			TokenType::QuestionMark,
			TokenType::ExclamationPoint,
		]) {
			if tokens.next_is(TokenType::QuestionMark) {
				end = tokens.pop(TokenType::QuestionMark)?.span;
				return Ok(Expression::Unary(UnaryOperation {
					expression: Box::new(expression),
					operator: UnaryOperator::QuestionMark,
					span: start.to(end),
				}));
			}

			// Compile-time arguments
			let (compile_time_arguments, has_keyword_compile_time_arguments) = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
				let mut compile_time_arguments = Vec::new();
				let mut has_compile_time_keyword_arguments = false;
				end = parse_list!(tokens, ListType::AngleBracketed, {
					// Keyword argument
					if tokens.next_is(TokenType::Identifier) && tokens.next_next_is(TokenType::Equal) {
						let name = Name::try_parse(tokens)?;
						let _ = tokens.pop(TokenType::Equal)?;
						let value = Expression::parse(tokens);
						compile_time_arguments.push(Argument::Keyword(name, value));
						has_compile_time_keyword_arguments = true
					}
					// Regular argument
					else {
						compile_time_arguments.push(Argument::Positional(Expression::parse(tokens)));
					}
				})
				.span;
				(composite_arguments(compile_time_arguments), has_compile_time_keyword_arguments)
			});

			// Arguments
			let (arguments, has_keyword_arguments) = if_then_else_default!(tokens.next_is(TokenType::LeftParenthesis), {
				let mut arguments = Vec::new();
				let mut has_keyword_arguments = false;
				end = parse_list!(tokens, ListType::Parenthesized, {
					// Keyword argument
					if tokens.next_is(TokenType::Identifier) && tokens.next_next_is(TokenType::Equal) {
						let name = Name::try_parse(tokens)?;
						let _ = tokens.pop(TokenType::Equal)?;
						let value = Expression::parse(tokens);
						arguments.push(Argument::Keyword(name, value));
						has_keyword_arguments = true;
					}
					// Regular argument
					else {
						arguments.push(Argument::Positional(Expression::parse(tokens)));
					}
				})
				.span;
				(composite_arguments(arguments), has_keyword_arguments)
			});

			// Reassign base expression
			expression = Expression::FunctionCall(FunctionCall {
				function: Box::new(expression),
				compile_time_arguments,
				arguments,
				scope_id: context().scope_data.unique_id(),
				span: start.to(end),
				tags: TagList::default(),
				has_keyword_arguments,
				has_keyword_compile_time_arguments,
			});
		}

		Ok(expression)
	}
}

impl CompileTime for FunctionCall {
	type Output = Expression;

	fn evaluate_at_compile_time(mut self) -> anyhow::Result<Self::Output> {
		let debug_section = debug_start!("{} a {} at compile-time", "Compile-Time Evaluating".bold().green(), "function call".cyan());

		self.tags = self.tags.evaluate_at_compile_time()?;

		debug_log!("Evaluating the function to call in a {}", "function call".cyan());
		let function = self.function.evaluate_at_compile_time().map_err(mapped_err! {
			while = "evaluating the function to call on a function-call expression at compile-time",
		})?;

		// Compile-time arguments
		let builtin = context()
			.scope_data
			.get_variable_from_id("builtin", ScopeData::get_stdlib_id())
			.unwrap()
			.try_as::<VirtualPointer>()?;
		let compile_time_arguments = if function.try_as::<VirtualPointer>().is_ok_and(|pointer| pointer == builtin) {
			let object: ObjectConstructor = VecDeque::from(self.compile_time_arguments).pop_front().unwrap().try_into().unwrap();

			vec![Expression::Pointer(
				LiteralObject {
					internal_fields: object.internal_fields,
					address: None,
					field_access_type: FieldAccessType::Normal,
					fields: HashMap::new(),
					inner_scope_id: None,
					outer_scope_id: context().scope_data.unique_id(),
					span: Span::unknown(),
					tags: TagList::default(),
					type_name: "Text".into(),
					name: "anonymous_string_literal".into(),
				}
				.store_in_memory(),
			)]
		} else {
			let compile_args_debug = debug_start!(
				"{} a {} compile-time arguments at compile-time",
				"Compile-Time Evaluating".bold().green(),
				"function call's".cyan()
			);
			let mut evaluated_compile_time_arguments = Vec::new();
			for compile_time_argument in self.compile_time_arguments {
				debug_log!(
					"Evaluating compile-time argument {} of a {}",
					evaluated_compile_time_arguments.len() + 1,
					"function call".cyan()
				);
				let evaluated = compile_time_argument.evaluate_at_compile_time().map_err(mapped_err! {
					while = "evaluating a function call's compile-time argument at compile-time",
				})?;
				evaluated_compile_time_arguments.push(evaluated);
			}
			compile_args_debug.finish();
			evaluated_compile_time_arguments
		};

		// Arguments
		let mut arguments = {
			let arguments_debug = debug_start!("{} a {} arguments at compile-time", "Compile-Time Evaluating".bold().green(), "function call's".cyan());
			let mut evaluated_arguments = Vec::new();
			for argument in self.arguments {
				let evaluated = argument.evaluate_at_compile_time().map_err(mapped_err! {
					while = "evaluating a function call's argument at compile-time",
				})?;
				evaluated_arguments.push(evaluated);
			}
			arguments_debug.finish();
			evaluated_arguments
		};

		// If not all arguments are known at compile-time, we can't call the function at compile time. In this case, we just
		// return a function call expression, and it'll get transpiled to C and called at runtime.
		if arguments
			.iter()
			.map(|argument| argument.is_fully_known_at_compile_time())
			.collect::<anyhow::Result<Vec<_>>>()?
			.iter()
			.any(|value| !value)
		{
			debug_section.finish();
			return Ok(Expression::FunctionCall(FunctionCall {
				function: Box::new(function),
				compile_time_arguments,
				arguments,
				scope_id: self.scope_id,
				span: self.span,
				tags: self.tags,
				has_keyword_arguments: self.has_keyword_arguments,
				has_keyword_compile_time_arguments: self.has_keyword_compile_time_arguments,
			}));
		}

		// Evaluate function
		let literal = function.try_as_literal();
		if let Ok(function_declaration) = literal {
			let function_declaration = FunctionDeclaration::from_literal(function_declaration).map_err(mapped_err! {
				while = "interpreting a literal as a function declaration at compile-time",
			})?;

			// Set this object
			if let Some(this_object) = function_declaration.this_object() {
				if let Some(parameter) = function_declaration.parameters().first() {
					if parameter.name().unmangled_name() == "this" {
						debug_log!("{} the \"this object\" of a {}", "Compile-Time Evaluating".green().bold(), "function call".cyan());
						arguments.insert(0, this_object.clone().evaluate_at_compile_time()?);
					}
				}
			}

			// Keyword arguments
			if !self.has_keyword_arguments && function_declaration.parameters().last().is_some_and(|parameter| parameter.name() == &"options".into()) {
				let options_type_name = function_declaration
					.parameters()
					.last()
					.unwrap()
					.parameter_type()
					.try_as::<VirtualPointer>()?
					.virtual_deref()
					.name()
					.clone();
				let options = ObjectConstructor {
					type_name: options_type_name,
					fields: Vec::new(),
					internal_fields: HashMap::new(),
					name: "options".into(),
					outer_scope_id: context().scope_data.unique_id(),
					inner_scope_id: context().scope_data.unique_id(),
					field_access_type: FieldAccessType::Normal,
					span: Span::unknown(),
					tags: TagList::default(),
				}
				.evaluate_at_compile_time()?;
				arguments.push(options);
			}

			// Validate compile-time arguments
			for (argument, parameter) in compile_time_arguments.iter().zip(function_declaration.compile_time_parameters().iter()) {
				let parameter_type_pointer = parameter.parameter_type().try_as_literal()?.address.as_ref().unwrap().to_owned();
				if !argument.is_assignable_to_type(parameter_type_pointer)? {
					bail_err! {
						base = format!(
							"Attempted to pass a argument of type \"{}\" to a compile-time parameter of type \"{}\"",
							argument.get_type()?.virtual_deref().name().unmangled_name().bold().cyan(),
							parameter_type_pointer.virtual_deref().name().unmangled_name().bold().cyan(),
						),
						while = "validating the arguments in a function call",
					};
				}
				if !argument.is_fully_known_at_compile_time()? {
					anyhow::bail!("Attempted to pass a value that's not fully known at compile-time to a compile-time parameter.");
				}
			}

			// Validate arguments
			for (argument, parameter) in arguments.iter().zip(function_declaration.parameters().iter()) {
				let parameter_type_pointer = parameter.parameter_type().try_as_literal()?.address.as_ref().unwrap().to_owned();
				if !argument.is_assignable_to_type(parameter_type_pointer)? {
					bail_err! {
						base = format!(
							"Attempted to pass a argument of type \"{}\" to a parameter of type \"{}\"",
							argument.get_type()?.virtual_deref().name().unmangled_name().bold().cyan(),
							parameter_type_pointer.virtual_deref().name().unmangled_name().bold().cyan(),
						),
						while = "validating the arguments in a function call",
						position = argument.span(),
					};
				}
			}

			// Non-builtin
			if let Some(body) = function_declaration.body() {
				let inner_debug_section = debug_start!("{} a {} body", "Compile-Time Evaluating".bold().green(), "function call".cyan());
				if let Expression::Block(block) = body {
					// Validate and add compile-time arguments
					for (argument, parameter) in compile_time_arguments.iter().zip(function_declaration.compile_time_parameters().iter()) {
						context().scope_data.reassign_variable_from_id(parameter.name(), argument.clone(), block.inner_scope_id())?;
					}

					// Validate and add arguments
					for (argument, parameter) in arguments.iter().zip(function_declaration.parameters().iter()) {
						context().scope_data.reassign_variable_from_id(parameter.name(), argument.clone(), block.inner_scope_id())?;
					}
				}

				// Return value
				let return_value = body.clone().evaluate_at_compile_time().map_err(mapped_err! {
					while = "calling a function at compile-time",
				})?;

				inner_debug_section.finish();

				// Return value is literal
				if return_value.try_as_literal().is_ok() {
					debug_log!(
						"{} compile-time evaluated into it's return value, which is a {}",
						"function call".cyan(),
						return_value.kind_name().cyan()
					);
					debug_section.finish();
					return Ok(return_value);
				}

				// Return value isn't literal
				debug_log!(
					"{} compile-time couldn't be evaluated into it's non-literal return value, which is a {}",
					"function call".cyan(),
					return_value.kind_name().cyan()
				);
			}
			// Builtin function
			else {
				let inner_debug_section = debug_start!("{} a built-in {}", "Compile-Time Evaluating", "function call".cyan());
				let mut builtin_name = None;
				let mut system_side_effects = false;
				let mut runtime = None;

				// Get the address of system_side_effects
				let system_side_effects_address = *context()
					.scope_data
					.get_variable_from_id("system_side_effects", ScopeData::get_stdlib_id())
					.unwrap()
					.try_as::<VirtualPointer>()
					.map_err(mapped_err! {
						while = format!("interpreting the global variable \"{}\" as a pointer", "system_side_effects".bold().cyan()),
					})?;

				// Get builtin and side effect tags
				for tag in &function_declaration.tags().values {
					if let Ok(object) = tag.try_as_literal() {
						if object.type_name() == &Name::from("BuiltinTag") {
							builtin_name = Some(
								object
									.get_field_literal("internal_name")
									.unwrap()
									.try_as::<String>()
									.map_err(mapped_err! {
										while = format!("interpreting the literal field \"{}\" of a {} as a string", "internal_name".bold().cyan(), "BuiltinTag".bold().cyan()),
									})?
									.to_owned(),
							);
							continue;
						}

						if tag.try_as::<VirtualPointer>().unwrap() == &system_side_effects_address {
							system_side_effects = true;
						}

						if let Ok(pointer) = tag.try_as::<VirtualPointer>() {
							let value = pointer.virtual_deref();
							if value.type_name() == &"RuntimeTag".into() {
								runtime = Some(value.get_field_literal("reason").unwrap().get_internal_field::<String>("internal_value")?);
							}
						}
					}
				}

				// Call builtin function
				if let Some(internal_name) = builtin_name {
					if !system_side_effects || context().has_side_effects() {
						if let Some(runtime_reason) = runtime {
							if !self.tags.suppresses_warning(CompilerWarning::RuntimeFunctionCall) {
								warn!(
									"The action {} was run at compile-time, but it should only be called at runtime. Reason: \n\n\t{} ",
									format!(
										"{}.{}()",
										regex!(r"^[^\.]+").find(&internal_name).unwrap().as_str().red(),
										regex!(r"\.(.+)").captures(&internal_name).unwrap().get(1).unwrap().as_str().blue()
									)
									.bold(),
									runtime_reason.dimmed()
								);
							}
						}

						let return_value = call_builtin_at_compile_time(&internal_name, self.scope_id, arguments, self.span).map_err(mapped_err! {
							while = format!("calling the built-in function {} at compile-time", internal_name.bold().purple()),
						});

						inner_debug_section.finish();
						debug_section.finish();
						return return_value;
					}

					inner_debug_section.finish();
					debug_section.finish();
					return Ok(Expression::ErrorExpression(Span::unknown()));
				}

				bail_err!(base = "Attempted to call a function that doesn't have a body.",);
			}
		}

		debug_section.finish();
		Ok(Expression::FunctionCall(FunctionCall {
			function: Box::new(function),
			compile_time_arguments,
			arguments,
			scope_id: self.scope_id,
			span: self.span,
			tags: self.tags,
			has_keyword_arguments: self.has_keyword_arguments,
			has_keyword_compile_time_arguments: self.has_keyword_compile_time_arguments,
		}))
	}
}

impl TranspileToC for FunctionCall {
	fn to_c(&self) -> anyhow::Result<String> {
		let function = FunctionDeclaration::from_literal(self.function.clone().evaluate_at_compile_time()?.try_as::<VirtualPointer>()?.virtual_deref())?;

		let return_type = if let Some(return_type) = function.return_type() {
			format!("{}* return_address;", return_type.try_as_literal()?.to_c_type()?)
		} else {
			String::new()
		};

		let ending_return_address = if let Some(_return_type) = function.return_type() {
			"return_address;".to_owned()
		} else {
			String::new()
		};

		let maybe_return_address = if let Some(_return_type) = function.return_type() {
			let maybe_comma = if function.parameters().is_empty() { "" } else { ", " };
			format!("{maybe_comma}return_address")
		} else {
			String::new()
		};

		Ok(unindent::unindent(&format!(
			"
			({{
				{return_type}	
				{argument_declaration}
				(((void (*)({parameter_types}))({function_to_call}->call))({this_object}{arguments}{maybe_return_address}));
				{ending_return_address}
			}})	
			",
			parameter_types = {
				let mut parameters = function
					.parameters()
					.iter()
					.map(|parameter| Ok(format!("{}*", parameter.parameter_type().try_as_literal()?.to_c_type()?)))
					.collect::<anyhow::Result<Vec<_>>>()?
					.join(", ");
				if let Some(function_return_type) = function.return_type().as_ref() {
					if !parameters.is_empty() {
						parameters += ", ";
					}
					write!(parameters, "{}*", function_return_type.try_as_literal()?.to_c_type()?).unwrap();
				}
				parameters
			},
			function_to_call = self.function.to_c()?,
			this_object = if let Some(object) = function.this_object() {
				if function.parameters().first().is_some_and(|param| param.name() == &"this".into()) {
					format!("{}, ", object.to_c()?)
				} else {
					String::new()
				}
			} else {
				String::new()
			},
			argument_declaration = self
				.arguments
				.iter()
				.map(|argument| Ok(format!("{}* arg0 = {};", argument.get_type()?.virtual_deref().to_c_type()?, argument.to_c()?)))
				.collect::<anyhow::Result<Vec<_>>>()?
				.join(", "),
			arguments = (0..self.arguments.len()).map(|index| format!("arg{index}")).collect::<Vec<_>>().join(", "),
		)))
	}
}

impl RuntimeableExpression for FunctionCall {
	fn evaluate_subexpressions_at_compile_time(self) -> anyhow::Result<Self> {
		let function = self.function.evaluate_at_compile_time().map_err(mapped_err! {
			while = "evaluating the function to call on a function-call expression at compile-time",
		})?;

		// Compile-time arguments
		let compile_time_arguments = {
			let mut compile_time_arguments = Vec::new();
			for argument in self.compile_time_arguments {
				let evaluated = argument.evaluate_at_compile_time()?;
				compile_time_arguments.push(evaluated);
			}
			compile_time_arguments
		};

		// Arguments
		let arguments = {
			let mut arguments = Vec::new();
			for argument in self.arguments {
				let evaluated = argument.evaluate_at_compile_time()?;
				arguments.push(evaluated);
			}
			arguments
		};

		Ok(FunctionCall {
			function: Box::new(function),
			compile_time_arguments,
			arguments,
			scope_id: self.scope_id,
			tags: self.tags,
			span: self.span,
			has_keyword_arguments: self.has_keyword_arguments,
			has_keyword_compile_time_arguments: self.has_keyword_compile_time_arguments,
		})
	}
}

impl Typed for FunctionCall {
	fn get_type(&self) -> anyhow::Result<VirtualPointer> {
		let function = FunctionDeclaration::from_literal(self.function.try_as_literal()?)?;
		if let Some(return_type) = function.return_type() {
			return_type.try_as::<VirtualPointer>().cloned()
		} else {
			context().scope_data.get_variable("Nothing").unwrap().try_as::<VirtualPointer>().cloned()
		}
	}
}

impl Spanned for FunctionCall {
	fn span(&self) -> Span {
		self.span
	}
}

impl FunctionCall {
	/// Converts a binary operation expression into a function call. In Cabin, binary operations are just function calls, so the expression:
	///
	/// ```
	/// first + second
	/// ```
	///
	/// is equivalent to:
	///
	/// ```
	/// first.plus(second)
	/// ```
	///
	/// So, this function converts from the first form of that into the second. This is used by `operators::parse_binary_expression()` at
	/// parse-time to convert parsed binary expressions into function calls.
	///
	/// # Parameters
	///
	/// - `left` - The expression on the left of the binary expression
	/// - `right` - The expression on the right of the binary expression
	/// - `operation` - The token of the operation symbol
	/// - `context` - Global data about the compiler's state
	///
	/// # Returns
	///
	/// The function call object created from the binary expression.
	///
	/// # Errors
	///
	/// Only if the given token does not represent a valid binary operation. The given token must have a type of
	/// `TokenType::Plus`, `TokenType::Minus`, etc.
	pub fn from_binary_operation(left: Expression, right: Expression, operation: Token) -> Result<FunctionCall, crate::Diagnostic> {
		let function_name = match operation.token_type {
			TokenType::Asterisk => "times",
			TokenType::DoubleEquals => "equals",
			TokenType::ForwardSlash => "divided_by",
			TokenType::LessThan => "is_less_than",
			TokenType::GreaterThan => "is_greater_than",
			TokenType::Minus => "minus",
			TokenType::Plus => "plus",
			_ => panic!("Invalid binary expression token type"),
		};

		let start = left.span();
		let middle = operation.span;
		let end = right.span();

		Ok(FunctionCall {
			function: Box::new(Expression::FieldAccess(FieldAccess::new(
				left,
				Name::from(function_name),
				context().scope_data.unique_id(),
				start.to(middle),
			))),
			arguments: vec![right],
			compile_time_arguments: Vec::new(),
			scope_id: context().scope_data.unique_id(),
			span: start.to(end),
			tags: TagList::default(),
			has_keyword_arguments: false,
			has_keyword_compile_time_arguments: false,
		})
	}

	/// Calls the program's main function at compile-time. This is used during the build process to begin compile-time evaluation.
	///
	/// # Parameters
	///
	/// - `function` - The main function to call.
	/// - `scope_id` - The scope ID of the main function
	///
	/// # Returns
	///
	/// The returned value from the main function.
	///
	/// # Errors
	///
	/// If an error occurred while evaluating the function call at compile-time, the error is returned.
	pub fn call_main(function: Expression, scope_id: ScopeId) -> anyhow::Result<Expression> {
		FunctionCall {
			function: Box::new(function),
			compile_time_arguments: Vec::new(),
			arguments: Vec::new(),
			scope_id,
			span: Span::unknown(),
			tags: TagList::default(),
			has_keyword_compile_time_arguments: false,
			has_keyword_arguments: false,
		}
		.evaluate_at_compile_time()
		.map_err(mapped_err! {
			while = "running the program's main file at compile-time",
		})
	}

	pub fn basic(function: Expression) -> FunctionCall {
		FunctionCall {
			function: Box::new(function),
			arguments: Vec::new(),
			compile_time_arguments: Vec::new(),
			scope_id: context().scope_data.unique_id(),
			span: Span::unknown(),
			has_keyword_arguments: false,
			has_keyword_compile_time_arguments: false,
			tags: TagList::default(),
		}
	}
}
