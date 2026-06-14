use std::collections::VecDeque;

use derive_getters::Getters;

use crate::{
	Span,
	Spanned,
	api::{builtin::call_builtin_at_compile_time, context::Context, scope::ScopeRef, traits::TryAs as _},
	ast::expressions::{
		Expression,
		action::EvaluatedAction,
		binary::BinaryOperation,
		evaluated::EvaluatedExpression,
		field_access::{FieldAccess, FieldAccessTokens},
		identifier::Identifier,
		run::Runtimeable,
	},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct FunctionCall {
	callee: Expression,
	compile_time_arguments: Vec<Expression>,
	arguments: Vec<Expression>,
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
	type Output = Expression;

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

					expression = Expression::FunctionCall(Box::new(FunctionCall {
						callee: expression,
						compile_time_arguments,
						arguments: arguments.unwrap_or_default(),
						scope: context.scope.unique_id(),
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
					}));
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

					expression = Expression::FunctionCall(Box::new(FunctionCall {
						callee: expression,
						compile_time_arguments: Vec::new(),
						arguments,
						scope: context.scope.unique_id(),
						tokens: FunctionCallTokens {
							arguments: Some(ArgumentTokens {
								left_parenthesis,
								right_parenthesis,
							}),
							compile_time_arguments: None,
						},
					}));
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

		let span = self.span();

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
		let arguments = {
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
			let _ = context.call_stack.pop().unwrap();
			return EvaluatedExpression::FunctionCall(EvaluatedFunctionCall {
				callee: function,
				compile_time_arguments,
				arguments,
				scope: self.scope,
				tokens: self.tokens,
			})
			.store(context);
		}

		context.call_stack.push(EvaluatedFunctionCall {
			callee: function,
			compile_time_arguments: compile_time_arguments.clone(),
			arguments: arguments.clone(),
			scope: self.scope,
			tokens: self.tokens.clone(),
		});

		// callee is an action
		if let Some(action) = function.value(context).as_expression().and_then(|expr| expr.try_as::<EvaluatedAction>().ok()) {
			let action = action.clone();

			// validate compile-time arguments
			for (argument, _parameter) in compile_time_arguments.iter().zip(action.compile_time_parameters().iter()) {
				if !argument.is_fully_known(context) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: argument.span(),
						info: DiagnosticInfo::ExpressionUsedAsType,
					});
				}
			}

			let parameter_count = action.parameters().as_ref().map(|p| p.len()).unwrap_or_default();
			if parameter_count != arguments.len() {
				context.add_diagnostic(Diagnostic {
					span,
					info: DiagnosticInfo::IncorrectArgumentCount {
						expected: parameter_count,
						actual: arguments.len(),
					},
					file: context.file.clone(),
				});
			}

			let mut builtin_name = None;
			let mut runtime = None;

			// Get runtime tags
			if let Some(tags) = action.tags() {
				for tag in tags.values() {
					if let Some(object) = tag.value().value(context).as_object() {
						if *object.group() == context.ident("Builtin") {
							builtin_name = Some(
								object
									.get_field_value("internal_name")
									.unwrap()
									.value(context)
									.as_object()
									.unwrap()
									.get_text()
									.unwrap()
									.to_owned(),
							);

							continue;
						}

						if *object.group() == context.ident("Runtime") {
							runtime = Some(object.get_field_value("reason").unwrap().value(context).as_object().unwrap().get_text().unwrap().to_owned());
						}
					}
				}
			}

			if let Some(reason) = runtime {
				context.add_diagnostic(Diagnostic {
					span,
					info: DiagnosticInfo::CallRuntimeAtCompileTime { reason },
					file: context.file.clone(),
				});
			}

			// non-builtin
			if let Some(block) = action.body() {
				// add compile-time arguments
				if let Some(compile_time_parameters) = action.compile_time_parameters() {
					for (argument, parameter) in compile_time_arguments.iter().zip(compile_time_parameters.iter()) {
						context.scope.reassign_variable_from_id(parameter.name(), *argument, block.inner_scope_id());
					}
				}

				// arguments
				if let Some(parameters) = action.parameters() {
					for (argument, parameter) in arguments.iter().zip(parameters.iter()) {
						context.scope.reassign_variable_from_id(parameter.name(), *argument, block.inner_scope_id());
					}
				}

				// return value
				let return_value = block.clone().evaluate(context);

				let _ = context.call_stack.pop().unwrap();

				return return_value;
			}

			// call builtin function
			if let Some(internal_name) = builtin_name {
				let return_value = call_builtin_at_compile_time(&internal_name, context, arguments, span);
				let _ = context.call_stack.pop().unwrap();

				if let Some(outer) = context.call_stack.last()
					&& outer.callee == context.scope.get_variable("log").unwrap()
					&& internal_name == "system.terminal.write"
				{
					context.diagnostics_mut().last_mut().unwrap().span = outer.span();
				}

				return return_value;
			}

			unreachable!()
		}

		// callee is not an action
		context.add_diagnostic(Diagnostic {
			file: context.file.clone(),
			span,
			info: DiagnosticInfo::CallNonFunction(function.kind_name(context).to_owned()),
		});

		let _ = context.call_stack.pop().unwrap();
		EvaluatedExpression::FunctionCall(EvaluatedFunctionCall {
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
	fn span(&self) -> Span {
		let start = self.callee.span();

		if let Some(argument) = self.arguments.last() {
			return start.to(argument.span());
		}

		if let Some(argument) = self.compile_time_arguments.last() {
			return start.to(argument.span());
		}

		unreachable!()
	}
}

impl FunctionCall {
	pub fn from_binary_expression(left: Expression, operation: BinaryOperation, right: Expression, operator: Token, context: &mut Context) -> FunctionCall {
		FunctionCall {
			callee: Expression::FieldAccess(Box::new(FieldAccess {
				left,
				right: Identifier::synthetic(Token::synthetic(TokenType::Identifier, operation.action_name(), operator.span), context),
				tokens: FieldAccessTokens { operator },
			})),
			compile_time_arguments: Vec::new(),
			arguments: vec![right],
			scope: context.scope.unique_id(),
			tokens: FunctionCallTokens {
				arguments: None,
				compile_time_arguments: None,
			},
		}
	}
}

impl Runtimeable for FunctionCall {
	type Output = EvaluatedFunctionCall;

	fn evaluate_subexpressions(self, context: &mut Context) -> EvaluatedFunctionCall {
		EvaluatedFunctionCall {
			callee: self.callee.evaluate(context),
			compile_time_arguments: self.compile_time_arguments.evaluate(context),
			arguments: self.arguments.evaluate(context),
			scope: self.scope,
			tokens: self.tokens,
		}
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedFunctionCall {
	callee: Pointer,
	compile_time_arguments: Vec<Pointer>,
	arguments: Vec<Pointer>,
	scope: ScopeRef,
	tokens: FunctionCallTokens,
}

impl Spanned for EvaluatedFunctionCall {
	fn span(&self) -> Span {
		let start = self.callee.span();

		if let Some(argument) = self.arguments.last() {
			return start.to(argument.span());
		}

		if let Some(argument) = self.compile_time_arguments.last() {
			return start.to(argument.span());
		}

		unreachable!()
	}
}
