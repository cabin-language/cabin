use std::collections::VecDeque;

use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::{builtin::call_builtin_at_compile_time, context::Context, scope::ScopeRef, traits::TryAs as _},
	ast::expressions::{
		Expression,
		action::Action,
		binary::BinaryOperation,
		field_access::{FieldAccess, FieldAccessTokens},
		identifier::Identifier,
		run::Runtime,
	},
	comptime::{Evaluate, storage::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct FunctionCall {
	callee: Pointer,
	compile_time_arguments: Vec<Pointer>,
	arguments: Vec<Pointer>,
	scope: ScopeRef,
	tokens: FunctionCallTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct FunctionCallTokens {
	arguments: Option<ArgumentTokens>,
	compile_time_arguments: Option<CompileTimeArgumentTokens>,
}

#[derive(Debug, Clone, Getters)]
pub struct ArgumentTokens {
	left_parenthesis: Token,
	right_parenthesis: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct CompileTimeArgumentTokens {
	left_angle_bracket: Token,
	right_angle_bracket: Token,
}

pub struct PostfixExpression;

impl TryParse for PostfixExpression {
	type Output = Pointer;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// Primary expression
		let mut expression = FieldAccess::try_parse(tokens, context)?;

		loop {
			match tokens.peek_type(context)? {
				TokenType::LeftAngleBracket => {
					let mut compile_time_arguments = Vec::new();
					let left_angle_bracket = tokens.pop(TokenType::LeftAngleBracket, context).unwrap();
					while !tokens.next_is(TokenType::RightAngleBracket) {
						compile_time_arguments.push(Expression::try_parse(tokens, context)?);
						if tokens.pop(TokenType::Comma, context).is_err() {
							break;
						}
					}
					let right_angle_bracket = tokens.pop(TokenType::RightAngleBracket, context)?;

					let (arguments, parenthesis) = if_then_some!(tokens.next_is(TokenType::LeftParenthesis), {
						let mut arguments = Vec::new();
						let left_parenthesis = tokens.pop(TokenType::LeftParenthesis, context).unwrap();
						while !tokens.next_is(TokenType::RightParenthesis) {
							arguments.push(Expression::try_parse(tokens, context)?);
							if tokens.pop(TokenType::Comma, context).is_err() {
								break;
							}
						}
						let right_parenthesis = tokens.pop(TokenType::RightParenthesis, context)?;
						(arguments, (left_parenthesis, right_parenthesis))
					})
					.unzip();

					expression = Expression::FunctionCall(FunctionCall {
						callee: expression,
						compile_time_arguments,
						arguments: arguments.unwrap_or_default(),
						scope: context.scope.get_ref(),
						tokens: FunctionCallTokens {
							arguments: parenthesis.map(|(left_parenthesis, right_parenthesis)| ArgumentTokens {
								left_parenthesis,
								right_parenthesis,
							}),
							compile_time_arguments: Some(CompileTimeArgumentTokens {
								left_angle_bracket,
								right_angle_bracket,
							}),
						},
					})
					.store(context);
				},

				TokenType::LeftParenthesis => {
					let mut arguments = Vec::new();
					let left_parenthesis = tokens.pop(TokenType::LeftParenthesis, context).unwrap();
					while !tokens.next_is(TokenType::RightParenthesis) {
						arguments.push(Expression::try_parse(tokens, context)?);
						if tokens.pop(TokenType::Comma, context).is_err() {
							break;
						}
					}
					let right_parenthesis = tokens.pop(TokenType::RightParenthesis, context)?;

					expression = Expression::FunctionCall(FunctionCall {
						callee: expression,
						compile_time_arguments: Vec::new(),
						arguments,
						scope: context.scope.get_ref(),
						tokens: FunctionCallTokens {
							arguments: Some(ArgumentTokens {
								left_parenthesis,
								right_parenthesis,
							}),
							compile_time_arguments: None,
						},
					})
					.store(context);
				},

				_ => break,
			}
		}

		Ok(expression)
	}
}

impl Evaluate for FunctionCall {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating action call");

		let span = self.span().into_owned();

		let function = self.callee.evaluate(context);

		let compile_time_arguments = {
			let mut evaluated_compile_time_arguments = Vec::new();
			for compile_time_argument in self.compile_time_arguments {
				let evaluated = compile_time_argument.evaluate(context);
				evaluated_compile_time_arguments.push(evaluated);
			}
			evaluated_compile_time_arguments
		};

		// Arguments
		let mut arguments = {
			let mut evaluated_arguments = Vec::new();
			for argument in self.arguments {
				let evaluated = argument.evaluate(context);
				evaluated_arguments.push(evaluated);
			}
			evaluated_arguments
		};

		// If not all arguments are known at compile-time, we can't call the function at compile time. In this case, we just
		// return a function call expression, and it'll get transpiled to C and called at runtime.
		if arguments.iter().map(|argument| argument.is_fully_known(context)).any(|value| !value) {
			return Expression::FunctionCall(FunctionCall {
				callee: function,
				compile_time_arguments,
				arguments,
				scope: self.scope,
				tokens: self.tokens,
			})
			.store(context);
		}

		context.call_stack.push(FunctionCall {
			callee: function.to_owned(),
			compile_time_arguments: compile_time_arguments.clone(),
			arguments: arguments.clone(),
			scope: self.scope,
			tokens: self.tokens.clone(),
		});

		// callee is an action
		if let Some(action) = function.value(context).as_expression().and_then(|expr| expr.try_as::<Action>().ok()) {
			let action = action.clone();

			if let Some(this) = action.this() {
				arguments.insert(0, this.to_owned());
			}

			// validate compile-time arguments
			for (argument, _parameter) in compile_time_arguments.iter().zip(action.compile_time_parameters().iter()) {
				if !argument.is_fully_known(context) {
					context.add_diagnostic(Diagnostic::at(argument, DiagnosticInfo::ExpressionUsedAsType));
				}
			}

			let parameter_count = action.parameters().as_ref().map(|p| p.len()).unwrap_or_default();
			if parameter_count != arguments.len() {
				context.add_diagnostic(Diagnostic::at(&span, DiagnosticInfo::IncorrectArgumentCount {
					expected: parameter_count,
					actual: arguments.len(),
				}));
			}

			let mut builtin_name: Option<String> = None;
			let mut runtime: Option<String> = None;

			// Get runtime tags
			if let Some(tags) = action.tags() {
				for tag in tags.values() {
					if let Some(Expression::New(new)) = tag.value().value(context).as_expression() {
						// if new
						// 	.type_name()
						// 	.as_ref()
						// 	.is_some_and(|group| group.value(context).is_some_and(|value| value == context.builtin("BuiltinTag")))
						// {
						// 	builtin_name = Some(
						// 		new.fields()
						// 			.iter()
						// 			.find(|field| field.name().source_identifier() == "internal_name")
						// 			.unwrap()
						// 			.value()
						// 			.as_text_literal(context)
						// 			.unwrap()
						// 			.to_owned(),
						// 	);

						// 	continue;
						// }

						// if new
						// 	.type_name()
						// 	.as_ref()
						// 	.is_some_and(|group| group.value(context).is_some_and(|value| value == context.builtin("Runtime")))
						// {
						// 	runtime = Some(
						// 		new.fields()
						// 			.iter()
						// 			.find(|field| field.name().source_identifier() == "reason")
						// 			.unwrap()
						// 			.value()
						// 			.as_text_literal(context)
						// 			.unwrap()
						// 			.to_owned(),
						// 	)
						// }
					}
				}
			}

			if let Some(reason) = runtime {
				context.add_diagnostic(Diagnostic::at(&span, DiagnosticInfo::CallRuntimeAtCompileTime { reason }));
			}

			// non-builtin
			if let Some(block) = action.body() {
				// add compile-time arguments
				if let Some(compile_time_parameters) = action.compile_time_parameters() {
					for (argument, parameter) in compile_time_arguments.iter().zip(compile_time_parameters.iter()) {
						context.reassign_variable(parameter.name(), argument.to_owned());
					}
				}

				// arguments
				if let Some(parameters) = action.parameters() {
					for (argument, parameter) in arguments.iter().zip(parameters.iter()) {
						context.reassign_variable(parameter.name(), argument.to_owned());
					}
				}

				// return value
				let return_value = block.clone().evaluate(context);

				let _ = context.call_stack.pop().unwrap();

				return return_value;
			}

			// call builtin function
			if let Some(internal_name) = builtin_name {
				let return_value = call_builtin_at_compile_time(&internal_name, context, arguments.into(), &span);
				let _ = context.call_stack.pop().unwrap();

				if let Some(outer) = context.call_stack.last()
					&& internal_name == "system.terminal.write"
				{
					let outer_span = outer.span().into_owned();
					if outer.callee.clone() == context.get_builtin("log").unwrap() {
						context.diagnostics_mut().last_mut().unwrap().span = outer_span.into_owned();
					}
				}

				return return_value;
			}

			context.add_diagnostic(Diagnostic::at(&action, DiagnosticInfo::ActionWithoutBody));

			let _ = context.call_stack.pop().unwrap();
			return Expression::FunctionCall(FunctionCall {
				callee: function,
				compile_time_arguments,
				arguments,
				scope: self.scope,
				tokens: self.tokens,
			})
			.store(context);
		}

		// callee is not an action
		context.add_diagnostic(Diagnostic::at(
			&span,
			DiagnosticInfo::CallNonFunction(
				function
					.value(context)
					.as_expression()
					.map_or_else(|| "object".to_owned(), |expression| expression.kind_name().to_owned()),
			),
		));

		let _ = context.call_stack.pop().unwrap();
		Expression::FunctionCall(FunctionCall {
			callee: function,
			compile_time_arguments,
			arguments,
			scope: self.scope,
			tokens: self.tokens,
		})
		.store(context)
	}
}

impl Spanned for FunctionCall {
	fn span(&self) -> impl SpanLike {
		let start = self.callee.span();

		if let Some(arguments) = self.tokens().arguments() {
			return start.to(arguments.right_parenthesis().span());
		}

		if let Some(arguments) = self.tokens().compile_time_arguments() {
			return start.to(arguments.right_angle_bracket().span());
		}

		unreachable!()
	}
}

impl FunctionCall {
	pub fn from_binary_expression(left: Pointer, operation: BinaryOperation, right: Pointer, operator: Token, context: &mut Context) -> FunctionCall {
		let token = Token::synthetic(TokenType::Identifier, operation.action_name().to_owned(), operator.span().owned());
		FunctionCall {
			callee: Expression::FieldAccess(FieldAccess {
				left,
				right: Identifier::synthetic(token, context),
				tokens: FieldAccessTokens { operator },
			})
			.store(context),
			compile_time_arguments: Vec::new(),
			arguments: vec![right],
			scope: context.scope.get_ref(),
			tokens: FunctionCallTokens {
				arguments: None,
				compile_time_arguments: None,
			},
		}
	}
}

impl Runtime for FunctionCall {
	fn evaluate_subexpressions(self, context: &mut Context) -> Self {
		FunctionCall {
			callee: self.callee.evaluate(context),
			compile_time_arguments: self.compile_time_arguments.evaluate(context),
			arguments: self.arguments.evaluate(context),
			scope: self.scope,
			tokens: self.tokens,
		}
	}
}
