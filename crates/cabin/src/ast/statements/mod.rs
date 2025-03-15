use crate::{
	api::context::Context,
	ast::{
		expressions::Expression,
		statements::{declaration::Declaration, tail::TailStatement},
	},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::Diagnostic,
	interpreter::Runtime,
	io::{IoReader, IoWriter},
	lexer::TokenType,
	parser::{Parse, TokenQueue, TokenQueueFunctionality as _, TryParse as _},
	transpiler::{TranspileError, TranspileToC},
	Span,
	Spanned,
};

pub mod declaration;
pub mod tail;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Statement {
	Declaration(Declaration),
	Tail(TailStatement),
	Expression(ExpressionPointer),
	Error(Span),
}

impl Parse for Statement {
	type Output = Statement;

	fn parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Self::Output {
		fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Result<Statement, Diagnostic> {
			let statement = match tokens.peek_type(context)? {
				TokenType::KeywordLet | TokenType::TagOpening => Declaration::try_parse(tokens, context)?,
				TokenType::Identifier => {
					if tokens.peek_type2(context)? == TokenType::KeywordIs {
						let tail = Statement::Tail(TailStatement::try_parse(tokens, context)?);
						let _ = tokens.pop(TokenType::Semicolon, context)?;
						tail
					} else {
						let expression = Statement::Expression(Expression::parse(tokens, context));
						let _ = tokens.pop(TokenType::Semicolon, context)?;
						expression
					}
				},
				_ => {
					let expression = Statement::Expression(Expression::parse(tokens, context));
					let _ = tokens.pop(TokenType::Semicolon, context)?;
					expression
				},
			};
			Ok(statement)
		}

		let start = tokens.front().unwrap().span;
		match try_parse(tokens, context) {
			Ok(statement) => statement,
			Err(error) => {
				context.add_diagnostic(error);
				while let Ok(token_type) = tokens.peek_type(context) {
					if token_type == TokenType::Semicolon {
						let _ = tokens.pop(TokenType::Semicolon, context).unwrap();
						break;
					}

					let _ = tokens.pop(token_type, context).unwrap();
				}
				let end = tokens.front().map_or(Span::unknown(), |front| front.span);
				Statement::Error(start.to(end))
			},
		}
	}
}

impl CompileTime for Statement {
	type Output = Statement;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		match self {
			Statement::Declaration(declaration) => Statement::Declaration(declaration.evaluate_at_compile_time(context)),
			Statement::Expression(expression) => Statement::Expression(expression.evaluate_at_compile_time(context)),
			Statement::Tail(tail) => Statement::Tail(tail.evaluate_at_compile_time(context)),
			Statement::Error(span) => Statement::Error(span),
		}
	}
}

impl Runtime for Statement {
	type Output = Statement;

	fn evaluate_at_runtime<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		match self {
			Statement::Declaration(declaration) => Statement::Declaration(declaration.evaluate_at_runtime(context)),
			Statement::Expression(expression) => Statement::Expression(expression.evaluate_at_runtime(context)),
			Statement::Tail(tail) => Statement::Tail(tail.evaluate_at_runtime(context)),
			Statement::Error(span) => Statement::Error(span),
		}
	}
}

impl TranspileToC for Statement {
	fn to_c<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>, _output: Option<String>) -> Result<String, TranspileError> {
		Ok(match self {
			Statement::Declaration(declaration) => declaration.to_c(context, None)?,
			Statement::Tail(tail) => tail.to_c(context, None)?,
			Statement::Expression(expression) => expression.to_c(context, None)?,
			Statement::Error(_) => return Err(TranspileError::TranspileError),
		})
	}
}

impl Spanned for Statement {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &Context<Input, Output, Error>) -> Span {
		match self {
			Self::Declaration(declaration) => declaration.span(context),
			Self::Tail(tail) => tail.span(context),
			Self::Expression(expression) => expression.span(context),
			Self::Error(span) => *span,
		}
	}
}
