use crate::{
	api::context::context,
	comptime::CompileTime,
	lexer::{Span, TokenType},
	parser::{
		expressions::{Expression, Spanned},
		statements::{declaration::Declaration, tail::TailStatement},
		Parse,
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
	transpiler::TranspileToC,
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
	fn span(&self) -> Span {
		match self {
			Self::Declaration(declaration) => declaration.span(),
			Self::Tail(tail) => tail.span(),
			Self::Expression(expression) => expression.span(),
			Self::Error(span) => *span,
		}
	}
}

impl Parse for Statement {
	type Output = Statement;

	fn parse(tokens: &mut TokenQueue) -> Self::Output {
		fn try_parse(tokens: &mut TokenQueue) -> Result<Statement, crate::Diagnostic> {
			let statement = match tokens.peek_type()? {
				TokenType::KeywordLet | TokenType::TagOpening => Declaration::try_parse(tokens)?,
				TokenType::Identifier => {
					if tokens.peek_type2()? == TokenType::KeywordIs {
						let tail = Statement::Tail(TailStatement::try_parse(tokens)?);
						let _ = tokens.pop(TokenType::Semicolon)?;
						tail
					} else {
						let expression = Statement::Expression(Expression::parse(tokens));
						let _ = tokens.pop(TokenType::Semicolon)?;
						expression
					}
				},
				_ => {
					let expression = Statement::Expression(Expression::parse(tokens));
					let _ = tokens.pop(TokenType::Semicolon)?;
					expression
				},
			};
			Ok(statement)
		}

		let start = tokens.front().unwrap().span;
		match try_parse(tokens) {
			Ok(statement) => statement,
			Err(error) => {
				context().add_diagnostic(error);
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

	fn evaluate_at_compile_time(self) -> Self::Output {
		match self {
			Statement::Declaration(declaration) => Statement::Declaration(declaration.evaluate_at_compile_time()),
			Statement::Expression(expression) => Statement::Expression(expression.evaluate_at_compile_time()),
			Statement::Tail(tail) => Statement::Tail(tail.evaluate_at_compile_time()),
			Statement::Error(span) => Statement::Error(span),
		}
	}
}

impl TranspileToC for Statement {
	fn to_c(&self) -> anyhow::Result<String> {
		Ok(match self {
			Statement::Declaration(declaration) => declaration.to_c()?,
			Statement::Tail(tail_statement) => tail_statement.to_c()?,
			Statement::Expression(expression) => expression.to_c()? + ";",
			_ => todo!(),
		})
	}
}
