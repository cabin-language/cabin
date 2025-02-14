use std::{
	borrow::Cow,
	collections::{HashMap, VecDeque},
};

use crate::{
	api::{builtin::call_builtin_at_compile_time, context::Context, scope::ScopeId, traits::TryAs as _},
	ast::{
		expressions::{
			field_access::{FieldAccess, FieldAccessType},
			function_declaration::FunctionDeclaration,
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::{Field, ObjectConstructor},
			run::RuntimeableExpression,
			unary::{UnaryOperation, UnaryOperator},
			Expression,
			Spanned,
			Typed,
		},
		misc::tag::TagList,
	},
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	lexer::{Span, Token, TokenType},
	parse_list,
	parser::{ListType, Parse as _, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone)]
enum Argument {
	Positional(Expression),
	Keyword(Name, Expression),
}

fn composite_arguments(arguments: Vec<Argument>, context: &mut Context) -> Vec<Expression> {
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
		outer_scope_id: context.scope_tree.unique_id(),
		inner_scope_id: context.scope_tree.unique_id(),
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

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// Primary expression
		let mut expression = FieldAccess::try_parse(tokens, context)?;
		let start = expression.span(context);
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
						let name = Name::try_parse(tokens, context)?;
						let _ = tokens.pop(TokenType::Equal)?;
						let value = Expression::parse(tokens, context);
						compile_time_arguments.push(Argument::Keyword(name, value));
						has_compile_time_keyword_arguments = true
					}
					// Regular argument
					else {
						compile_time_arguments.push(Argument::Positional(Expression::parse(tokens, context)));
					}
				})
				.span;
				(composite_arguments(compile_time_arguments, context), has_compile_time_keyword_arguments)
			});

			// Arguments
			let (arguments, has_keyword_arguments) = if_then_else_default!(tokens.next_is(TokenType::LeftParenthesis), {
				let mut arguments = Vec::new();
				let mut has_keyword_arguments = false;
				end = parse_list!(tokens, ListType::Parenthesized, {
					// Keyword argument
					if tokens.next_is(TokenType::Identifier) && tokens.next_next_is(TokenType::Equal) {
						let name = Name::try_parse(tokens, context)?;
						let _ = tokens.pop(TokenType::Equal)?;
						let value = Expression::parse(tokens, context);
						arguments.push(Argument::Keyword(name, value));
						has_keyword_arguments = true;
					}
					// Regular argument
					else {
						arguments.push(Argument::Positional(Expression::parse(tokens, context)));
					}
				})
				.span;
				(composite_arguments(arguments, context), has_keyword_arguments)
			});

			// Reassign base expression
			expression = Expression::FunctionCall(FunctionCall {
				function: Box::new(expression),
				compile_time_arguments,
				arguments,
				scope_id: context.scope_tree.unique_id(),
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

	fn evaluate_at_compile_time(mut self, context: &mut Context) -> Self::Output {
		let span = self.span(context);
		self.tags = self.tags.evaluate_at_compile_time(context);

		let function = self.function.evaluate_at_compile_time(context);

		// Compile-time arguments
		let builtin = context
			.scope_tree
			.get_variable_from_id("builtin_function", ScopeId::stdlib())
			.unwrap()
			.try_as::<VirtualPointer>()
			.unwrap_or(&VirtualPointer::ERROR)
			.to_owned();
		let compile_time_arguments = if function.try_as::<VirtualPointer>().is_ok_and(|pointer| *pointer == builtin) {
			let object: ObjectConstructor = VecDeque::from(self.compile_time_arguments).pop_front().unwrap().try_into().unwrap();

			vec![Expression::Pointer(
				LiteralObject {
					internal_fields: object.internal_fields,
					address: None,
					field_access_type: FieldAccessType::Normal,
					fields: HashMap::new(),
					inner_scope_id: None,
					outer_scope_id: context.scope_tree.unique_id(),
					span: Span::unknown(),
					tags: TagList::default(),
					type_name: "Text".into(),
					name: "anonymous_string_literal".into(),
				}
				.store_in_memory(context),
			)]
		} else {
			let mut evaluated_compile_time_arguments = Vec::new();
			for compile_time_argument in self.compile_time_arguments {
				let evaluated = compile_time_argument.evaluate_at_compile_time(context);
				evaluated_compile_time_arguments.push(evaluated);
			}
			evaluated_compile_time_arguments
		};

		// Arguments
		let mut arguments = {
			let mut evaluated_arguments = Vec::new();
			for argument in self.arguments {
				let evaluated = argument.evaluate_at_compile_time(context);
				evaluated_arguments.push(evaluated);
			}
			evaluated_arguments
		};

		// If not all arguments are known at compile-time, we can't call the function at compile time. In this case, we just
		// return a function call expression, and it'll get transpiled to C and called at runtime.
		if arguments.iter().map(|argument| argument.is_fully_known_at_compile_time(context)).any(|value| !value) {
			return Expression::FunctionCall(FunctionCall {
				function: Box::new(function),
				compile_time_arguments,
				arguments,
				scope_id: self.scope_id,
				span: self.span,
				tags: self.tags,
				has_keyword_arguments: self.has_keyword_arguments,
				has_keyword_compile_time_arguments: self.has_keyword_compile_time_arguments,
			});
		}

		// Evaluate function
		let literal = function.try_as_literal(context);
		let function_declaration = literal;
		if !function_declaration.is_error() {
			if function_declaration.type_name() == &"Group".into() {
				return Expression::Pointer(function_declaration.address.unwrap());
			}

			let is_error = function_declaration.type_name() == &"Error".into();
			let function_declaration = FunctionDeclaration::from_literal(function_declaration).map(Cow::Owned).unwrap_or_else(|_| {
				if !is_error {
					context.add_diagnostic(Diagnostic {
						span,
						info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::CallNonFunction)),
					});
				}
				Cow::Borrowed(FunctionDeclaration::error())
			});

			// Set this object
			if let Some(this_object) = function_declaration.this_object() {
				if let Some(parameter) = function_declaration.parameters().first() {
					if parameter.name().unmangled_name() == "this" {
						arguments.insert(0, this_object.clone().evaluate_at_compile_time(context));
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
					.try_as::<VirtualPointer>()
					.unwrap_or(&VirtualPointer::ERROR)
					.virtual_deref(context)
					.name()
					.clone();
				let options = ObjectConstructor {
					type_name: options_type_name,
					fields: Vec::new(),
					internal_fields: HashMap::new(),
					name: "options".into(),
					outer_scope_id: context.scope_tree.unique_id(),
					inner_scope_id: context.scope_tree.unique_id(),
					field_access_type: FieldAccessType::Normal,
					span: Span::unknown(),
					tags: TagList::default(),
				}
				.evaluate_at_compile_time(context);
				arguments.push(options);
			}

			// Validate compile-time arguments
			// for (argument, parameter) in compile_time_arguments.iter().zip(function_declaration.compile_time_parameters().iter()) {
			// 	let parameter_type_pointer = parameter.parameter_type().try_as_literal().unwrap().address.as_ref().unwrap().to_owned();
			// 	// if !argument.is_assignable_to_type(parameter_type_pointer) {
			// 	// 	// bail_err! {
			// 	// 	// 	base = format!(
			// 	// 	// 		"Attempted to pass a argument of type \"{}\" to a compile-time parameter of type \"{}\"",
			// 	// 	// 		argument.get_type()?.virtual_deref().name().unmangled_name().bold().cyan(),
			// 	// 	// 		parameter_type_pointer.virtual_deref().name().unmangled_name().bold().cyan(),
			// 	// 	// 	),
			// 	// 	// 	while = "validating the arguments in a function call",
			// 	// 	// };
			// 	// }
			// 	// if !argument.is_fully_known_at_compile_time() {
			// 	// 	// anyhow::bail!("Attempted to pass a value that's not fully known at compile-time to a compile-time parameter.");
			// 	// }
			// }

			// Validate arguments
			// for (argument, parameter) in arguments.iter().zip(function_declaration.parameters().iter()) {
			// 	let parameter_type_pointer = parameter.parameter_type().try_as_literal()?.address.as_ref().unwrap().to_owned();
			// 	if !argument.is_assignable_to_type(parameter_type_pointer)? {
			// 		bail_err! {
			// 			base = format!(
			// 				"Attempted to pass a argument of type \"{}\" to a parameter of type \"{}\"",
			// 				argument.get_type()?.virtual_deref().name().unmangled_name().bold().cyan(),
			// 				parameter_type_pointer.virtual_deref().name().unmangled_name().bold().cyan(),
			// 			),
			// 			while = "validating the arguments in a function call",
			// 			position = argument.span(),
			// 		};
			// 	}
			// }

			// Non-builtin
			if let Some(body) = function_declaration.body() {
				if let Expression::Block(block) = body {
					// Validate and add compile-time arguments
					for (argument, parameter) in compile_time_arguments.iter().zip(function_declaration.compile_time_parameters().iter()) {
						context.scope_tree.reassign_variable_from_id(parameter.name(), argument.clone(), block.inner_scope_id());
					}

					// Validate and add arguments
					for (argument, parameter) in arguments.iter().zip(function_declaration.parameters().iter()) {
						context.scope_tree.reassign_variable_from_id(parameter.name(), argument.clone(), block.inner_scope_id());
					}
				}

				// Return value
				let return_value = body.clone().evaluate_at_compile_time(context);

				// Return value is literal
				if !return_value.try_as_literal(context).is_error() {
					return return_value;
				}
			}
			// Builtin function
			else {
				let mut builtin_name = None;
				let mut system_side_effects = false;
				let mut runtime = None;

				// Get the address of system_side_effects
				let system_side_effects_address = *context
					.scope_tree
					.get_variable_from_id("system_side_effects", ScopeId::stdlib())
					.unwrap()
					.try_as::<VirtualPointer>()
					.unwrap_or(&VirtualPointer::ERROR);

				// Get builtin and side effect tags
				for tag in &function_declaration.tags().values {
					let object = tag.try_as_literal(context);
					if !object.is_error() {
						if object.type_name() == &Name::from("BuiltinTag") {
							builtin_name = Some(
								object
									.get_field_literal("internal_name")
									.unwrap()
									.virtual_deref(context)
									.try_as::<String>()
									.unwrap()
									.to_owned(),
							);
							continue;
						}

						if tag.try_as::<VirtualPointer>().unwrap() == &system_side_effects_address {
							system_side_effects = true;
						}

						if let Ok(pointer) = tag.try_as::<VirtualPointer>() {
							let value = pointer.virtual_deref(context);
							if value.type_name() == &"RuntimeTag".into() {
								runtime = Some(
									value
										.get_field_literal("reason")
										.unwrap()
										.virtual_deref(context)
										.get_internal_field::<String>("internal_value")
										.unwrap()
										.to_owned(),
								);
							}
						}
					}
				}

				// Call builtin function
				if let Some(internal_name) = builtin_name {
					if !system_side_effects || context.has_side_effects() {
						if let Some(_runtime_reason) = runtime {
							// TODO: runtime tag
						}

						let return_value = call_builtin_at_compile_time(&internal_name, context, self.scope_id, arguments, self.span);
						return return_value;
					}

					return Expression::ErrorExpression(Span::unknown());
				}

				return Expression::ErrorExpression(span);
			}
		}

		Expression::FunctionCall(FunctionCall {
			function: Box::new(function),
			compile_time_arguments,
			arguments,
			scope_id: self.scope_id,
			span: self.span,
			tags: self.tags,
			has_keyword_arguments: self.has_keyword_arguments,
			has_keyword_compile_time_arguments: self.has_keyword_compile_time_arguments,
		})
	}
}

impl RuntimeableExpression for FunctionCall {
	fn evaluate_subexpressions_at_compile_time(self, context: &mut Context) -> Self {
		let function = self.function.evaluate_at_compile_time(context);

		// Compile-time arguments
		let compile_time_arguments = {
			let mut compile_time_arguments = Vec::new();
			for argument in self.compile_time_arguments {
				let evaluated = argument.evaluate_at_compile_time(context);
				compile_time_arguments.push(evaluated);
			}
			compile_time_arguments
		};

		// Arguments
		let arguments = {
			let mut arguments = Vec::new();
			for argument in self.arguments {
				let evaluated = argument.evaluate_at_compile_time(context);
				arguments.push(evaluated);
			}
			arguments
		};

		FunctionCall {
			function: Box::new(function),
			compile_time_arguments,
			arguments,
			scope_id: self.scope_id,
			tags: self.tags,
			span: self.span,
			has_keyword_arguments: self.has_keyword_arguments,
			has_keyword_compile_time_arguments: self.has_keyword_compile_time_arguments,
		}
	}
}

impl Typed for FunctionCall {
	fn get_type(&self, context: &mut Context) -> anyhow::Result<VirtualPointer> {
		let function = FunctionDeclaration::from_literal(self.function.try_as_literal(context))?;
		if let Some(return_type) = function.return_type() {
			return_type.try_as::<VirtualPointer>().cloned()
		} else {
			context.scope_tree.get_variable("Nothing").unwrap().try_as::<VirtualPointer>().cloned()
		}
	}
}

impl Spanned for FunctionCall {
	fn span(&self, _context: &Context) -> Span {
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
	pub fn from_binary_operation(context: &mut Context, left: Expression, right: Expression, operation: Token) -> Result<FunctionCall, Diagnostic> {
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

		let start = left.span(context);
		let middle = operation.span;
		let end = right.span(context);

		Ok(FunctionCall {
			function: Box::new(Expression::FieldAccess(FieldAccess::new(
				left,
				Name::from(function_name),
				context.scope_tree.unique_id(),
				start.to(middle),
			))),
			arguments: vec![right],
			compile_time_arguments: Vec::new(),
			scope_id: context.scope_tree.unique_id(),
			span: start.to(end),
			tags: TagList::default(),
			has_keyword_arguments: false,
			has_keyword_compile_time_arguments: false,
		})
	}

	pub fn basic(function: Expression, context: &mut Context) -> FunctionCall {
		FunctionCall {
			function: Box::new(function),
			arguments: Vec::new(),
			compile_time_arguments: Vec::new(),
			scope_id: context.scope_tree.unique_id(),
			span: Span::unknown(),
			has_keyword_arguments: false,
			has_keyword_compile_time_arguments: false,
			tags: TagList::default(),
		}
	}
}
