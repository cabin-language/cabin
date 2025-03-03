use std::collections::VecDeque;

use super::{new_literal::Literal, ExpressionOrPointer};
use crate::{
	api::{builtin::call_builtin_at_compile_time, context::Context, scope::ScopeId, traits::TryAs as _},
	ast::{
		expressions::{
			field_access::FieldAccess,
			function_declaration::EvaluatedFunctionDeclaration,
			name::Name,
			new_literal::Object,
			run::RuntimeableExpression,
			unary::{UnaryOperation, UnaryOperator},
			Expression,
		},
		misc::tag::TagList,
		sugar::string::CabinString,
	},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
		CompileTimeError,
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	lexer::{Token, TokenType},
	parse_list,
	parser::{ListType, Parse as _, TokenQueueFunctionality as _, TryParse},
	typechecker::Typed as _,
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct FunctionCall {
	function: ExpressionPointer,
	compile_time_arguments: Vec<ExpressionPointer>,
	arguments: Vec<ExpressionPointer>,
	scope_id: ScopeId,
	span: Span,
	tags: TagList,
}

pub struct EvaluatedFunctionCall {
	function: LiteralPointer,
	compile_time_arguments: Vec<LiteralPointer>,
	arguments: Vec<ExpressionPointer>,
	span: Span,
	tags: TagList,
}

pub(crate) struct PostfixOperators;

impl TryParse for PostfixOperators {
	type Output = ExpressionPointer;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// Primary expression
		let mut expression = FieldAccess::try_parse(tokens, context)?;
		let start = expression.span(context);
		let mut end = start;

		// Postfix function call operators
		while tokens.next_is_one_of(
			&[
				TokenType::LeftParenthesis,
				TokenType::LeftAngleBracket,
				TokenType::QuestionMark,
				TokenType::ExclamationPoint,
			],
			context,
		) {
			if tokens.next_is(TokenType::QuestionMark, context) {
				end = tokens.pop(TokenType::QuestionMark, context)?.span;
				return Ok(Expression::Unary(UnaryOperation {
					expression,
					operator: UnaryOperator::QuestionMark,
					span: start.to(end),
				})
				.store_in_memory(context));
			}

			// Compile-time arguments
			let compile_time_arguments = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket, context), {
				let mut compile_time_arguments = Vec::new();
				end = parse_list!(tokens, context, ListType::AngleBracketed, {
					compile_time_arguments.push(Expression::parse(tokens, context));
				})
				.span;
				compile_time_arguments
			});

			// Arguments
			let arguments = if_then_else_default!(tokens.next_is(TokenType::LeftParenthesis, context), {
				let mut arguments = Vec::new();
				end = parse_list!(tokens, context, ListType::Parenthesized, {
					arguments.push(Expression::parse(tokens, context));
				})
				.span;
				arguments
			});

			// Reassign base expression
			expression = Expression::FunctionCall(FunctionCall {
				function: expression,
				compile_time_arguments,
				arguments,
				scope_id: context.scope_tree.unique_id(),
				span: start.to(end),
				tags: TagList::default(),
			})
			.store_in_memory(context);
		}

		Ok(expression)
	}
}

impl CompileTime for FunctionCall {
	type Output = ExpressionOrPointer;

	fn evaluate_at_compile_time(mut self, context: &mut Context) -> Self::Output {
		let span = self.function.span(context);
		self.tags = self.tags.evaluate_at_compile_time(context);

		let function = self.function.evaluate_at_compile_time(context);

		let compile_time_arguments = {
			let mut evaluated_compile_time_arguments = Vec::new();
			for compile_time_argument in self.compile_time_arguments {
				let evaluated = compile_time_argument.evaluate_at_compile_time(context);
				evaluated_compile_time_arguments.push(evaluated);
			}
			evaluated_compile_time_arguments
		};

		// Arguments
		let arguments = {
			let mut evaluated_arguments = Vec::new();
			for argument in self.arguments {
				let evaluated = argument.evaluate_at_compile_time(context);
				evaluated_arguments.push(evaluated);
			}
			evaluated_arguments
		};

		// If not all arguments are known at compile-time, we can't call the function at compile time. In this case, we just
		// return a function call expression, and it'll get transpiled to C and called at runtime.
		if arguments.iter().map(|argument| argument.is_literal(context)).any(|value| !value) {
			return ExpressionOrPointer::Expression(Expression::FunctionCall(FunctionCall {
				function,
				compile_time_arguments,
				arguments,
				scope_id: self.scope_id,
				span: self.span,
				tags: self.tags,
			}));
		}

		// Evaluate function
		if let Ok(pointer) = function.try_as_literal(context) {
			let literal = pointer.get_literal(context).to_owned();
			let function_declaration = literal.try_as::<EvaluatedFunctionDeclaration>().unwrap_or_else(|_error| {
				if !matches!(literal, Literal::ErrorLiteral(_)) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span,
						info: CompileTimeError::CallNonFunction.into(),
					});
				}
				EvaluatedFunctionDeclaration::error()
			});

			//
			// // Set this object
			// if let Some(this_object) = function_declaration.this_object() {
			// 	if let Some(parameter) = function_declaration.parameters().first() {
			// 		if parameter.name().unmangled_name() == "this" {
			// 			arguments.insert(0, this_object.clone().evaluate_at_compile_time(context));
			// 		}
			// 	}
			// }

			// Validate compile-time arguments
			for (argument, parameter) in compile_time_arguments.iter().zip(function_declaration.compile_time_parameters().iter()) {
				// Typecheck that the argument is assignable to the parameter type
				let argument_type = argument.get_type(context);
				if !argument_type.is_assignable_to(parameter.parameter_type(), context) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: argument.span(context),
						info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::TypeMismatch(
							parameter.parameter_type().to_owned(),
							argument_type,
						))),
					});
				}

				// Argument to compile-time parameter must be known at compile-time
				if !argument.is_literal(context) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: argument.span(context),
						info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::ExpressionUsedAsType)),
					});
				}
			}

			// Validate arguments
			for (argument, parameter) in arguments.iter().zip(function_declaration.parameters().iter()) {
				let argument_type = argument.get_type(context);
				if !argument_type.is_assignable_to(parameter.parameter_type(), context) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: argument.span(context),
						info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::TypeMismatch(
							parameter.parameter_type().to_owned(),
							argument_type,
						))),
					});
				}
			}

			// Non-builtin
			if let Some(block) = function_declaration.body() {
				// Add compile-time arguments
				for (argument, parameter) in compile_time_arguments.iter().zip(function_declaration.compile_time_parameters().iter()) {
					context.scope_tree.reassign_variable_from_id(parameter.name(), argument.clone(), block.inner_scope_id());
				}

				// Add arguments
				for (argument, parameter) in arguments.iter().zip(function_declaration.parameters().iter()) {
					context.scope_tree.reassign_variable_from_id(parameter.name(), argument.clone(), block.inner_scope_id());
				}

				// Return value
				let return_value = block.clone().evaluate_at_compile_time(context);
				return ExpressionOrPointer::Expression(Expression::Block(return_value));
			}
			// Builtin function
			else {
				let mut builtin_name = None;
				let mut system_side_effects = false;
				let mut runtime = None;

				// Get the address of system_side_effects
				let system_side_effects_address = context
					.scope_tree
					.get_variable_from_id("system_side_effects", ScopeId::stdlib())
					.unwrap_or(ExpressionPointer::ERROR)
					.as_literal(context);

				// Get builtin and side effect tags
				for tag in &function_declaration.tags().values {
					if let Ok(literal) = tag.try_as_literal(context) {
						if literal == system_side_effects_address {
							system_side_effects = true;
							continue;
						}

						let object = literal.get_literal(context).try_as::<Object>().unwrap();
						if object.type_name() == &Name::from("BuiltinTag") {
							builtin_name = Some(
								object
									.get_field("internal_name")
									.unwrap()
									.get_literal(context)
									.try_as::<CabinString>()
									.unwrap()
									.value
									.to_owned(),
							);
							continue;
						}
					}

					if let Ok(pointer) = tag.try_as_literal(context) {
						if let Ok(object) = pointer.get_literal(context).try_as::<Object>() {
							if object.type_name() == &"RuntimeTag".into() {
								runtime = Some(
									object
										.get_field("reason")
										.unwrap()
										.get_literal(context)
										.try_as::<Object>()
										.unwrap()
										.get_field("internal_value")
										.unwrap()
										.get_literal(context)
										.try_as::<CabinString>()
										.unwrap()
										.value
										.to_owned(),
								);
							}
						}
					}
				}

				// Call builtin function
				if let Some(internal_name) = builtin_name {
					if !system_side_effects || context.side_effects {
						if let Some(_runtime_reason) = runtime {
							// TODO: runtime tag
						}

						return ExpressionOrPointer::Pointer(call_builtin_at_compile_time(&internal_name, context, self.scope_id, arguments, self.span));
					}

					return ExpressionOrPointer::Pointer(Expression::error(Span::unknown(), context));
				}

				return ExpressionOrPointer::Pointer(Expression::error(span, context));
			}
		}

		ExpressionOrPointer::Expression(Expression::FunctionCall(FunctionCall {
			function,
			compile_time_arguments,
			arguments,
			scope_id: self.scope_id,
			span: self.span,
			tags: self.tags,
		}))
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
			function,
			compile_time_arguments,
			arguments,
			scope_id: self.scope_id,
			tags: self.tags,
			span: self.span,
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
	/// ```cabin
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
	pub(crate) fn from_binary_operation(context: &mut Context, left: ExpressionPointer, right: ExpressionPointer, operation: Token) -> Result<FunctionCall, Diagnostic> {
		let function_name = match operation.token_type {
			TokenType::Asterisk => "times",
			TokenType::DoubleEquals => "equals",
			TokenType::ForwardSlash => "divided_by",
			TokenType::LessThan => "is_less_than",
			TokenType::GreaterThan => "is_greater_than",
			TokenType::Minus => "minus",
			TokenType::Plus => "plus",
			_ => unreachable!("Invalid binary expression token type"),
		};

		let start = left.span(context);
		let middle = operation.span;
		let end = right.span(context);

		Ok(FunctionCall {
			function: Expression::FieldAccess(FieldAccess::new(left, Name::from(function_name), context.scope_tree.unique_id(), start.to(middle))).store_in_memory(context),
			arguments: vec![right],
			compile_time_arguments: Vec::new(),
			scope_id: context.scope_tree.unique_id(),
			span: start.to(end),
			tags: TagList::default(),
		})
	}

	pub(crate) fn basic(function: ExpressionPointer, context: &mut Context) -> FunctionCall {
		FunctionCall {
			function,
			arguments: Vec::new(),
			compile_time_arguments: Vec::new(),
			scope_id: context.scope_tree.unique_id(),
			span: Span::unknown(),
			tags: TagList::default(),
		}
	}
}
