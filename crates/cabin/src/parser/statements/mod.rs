use crate::{
	api::context::Context,
	comptime::CompileTime,
	diagnostics::Diagnostic,
	lexer::{Span, TokenType},
	parser::{
		expressions::{Expression, Spanned},
		statements::{declaration::Declaration, tail::TailStatement},
		Parse,
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
};

pub mod declaration;
pub mod tag;
pub mod tail;

#[derive(Debug, Clone)]
pub enum Statement {
	Declaration(Declaration),
	Tail(TailStatement),
	Expression(Expression),
	Error(Span),
}

impl Spanned for Statement {
	fn span(&self, context: &Context) -> Span {
		match self {
			Self::Declaration(declaration) => declaration.span(context),
			Self::Tail(tail) => tail.span(context),
			Self::Expression(expression) => expression.span(context),
			Self::Error(span) => *span,
		}
	}
}

impl Parse for Statement {
	type Output = Statement;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output {
		fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Statement, Diagnostic> {
			let statement = match tokens.peek_type()? {
				TokenType::KeywordLet | TokenType::TagOpening => Declaration::try_parse(tokens, context)?,
				TokenType::Identifier => {
					if tokens.peek_type2()? == TokenType::KeywordIs {
						let tail = Statement::Tail(TailStatement::try_parse(tokens, context)?);
						let _ = tokens.pop(TokenType::Semicolon)?;
						tail
					} else {
						let expression = Statement::Expression(Expression::parse(tokens, context));
						let _ = tokens.pop(TokenType::Semicolon)?;
						expression
					}
				},
				_ => {
					let expression = Statement::Expression(Expression::parse(tokens, context));
					let _ = tokens.pop(TokenType::Semicolon)?;
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
				while let Ok(token_type) = tokens.peek_type() {
					if token_type == TokenType::Semicolon {
						let _ = tokens.pop(TokenType::Semicolon).unwrap();
						break;
					}

					let _ = tokens.pop(token_type).unwrap();
				}
				let end = tokens.front().unwrap().span;
				Statement::Error(start.to(end))
			},
		}
	}
}

impl CompileTime for Statement {
	type Output = Statement;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		match self {
			Statement::Declaration(declaration) => Statement::Declaration(declaration.evaluate_at_compile_time(context)),
			Statement::Expression(expression) => Statement::Expression(expression.evaluate_at_compile_time(context)),
			Statement::Tail(tail) => Statement::Tail(tail.evaluate_at_compile_time(context)),
			Statement::Error(span) => Statement::Error(span),
		}
	}
}
