use crate::{
	Span,
	SpanLike,
	Spanned,
	api::context::Context,
	ast::{
		expressions::Expression,
		statements::{declaration::Declaration, tail::TailStatement},
	},
	comptime::{Evaluate, storage::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::TokenType,
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

pub mod declaration;
pub mod tail;

#[derive(Debug, Clone)]
pub enum Statement {
	Declaration(Declaration),
	Tail(TailStatement),
	Expression(Pointer),
	Error(Span),
}

impl TryParse for Statement {
	type Output = Statement;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Statement, Diagnostic> {
			let statement = match tokens.peek_type(context)? {
				TokenType::KeywordLet | TokenType::TagOpening => Declaration::try_parse(tokens, context)?,
				TokenType::Identifier if tokens.peek_type2(context)? == TokenType::KeywordIs => {
					let tail = Statement::Tail(TailStatement::try_parse(tokens, context)?);
					let _ = tokens.pop(TokenType::Semicolon, context)?;
					tail
				},
				_ => {
					let expression = Statement::Expression(Expression::try_parse(tokens, context)?);
					let _ = tokens.pop(TokenType::Semicolon, context)?;
					expression
				},
			};
			Ok(statement)
		}

		match try_parse(tokens, context) {
			Ok(statement) => Ok(statement),
			Err(error) => {
				context.add_diagnostic(error);

				while let Ok(token_type) = tokens.peek_type(context) {
					if token_type == TokenType::Semicolon {
						let _ = tokens.pop(TokenType::Semicolon, context).unwrap();
						break;
					}

					let _ = tokens.pop(token_type, context).unwrap();
				}

				if tokens.is_empty() {
					Err(Diagnostic::at(&Span::eof(&context.file), DiagnosticInfo::UnexpectedGenericEOF))
				} else {
					Ok(Statement::Error(tokens.peek(context).unwrap().span().owned()))
				}
			},
		}
	}
}

impl Evaluate for Statement {
	type Output = Statement;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating statement");
		match self {
			Statement::Declaration(declaration) => Statement::Declaration(declaration.evaluate(context)),
			Statement::Expression(expression) => Statement::Expression(expression.evaluate(context)),
			Statement::Tail(tail) => Statement::Tail(tail.evaluate(context)),
			Statement::Error(span) => Statement::Error(span),
		}
	}
}

impl Spanned for Statement {
	fn span(&self) -> impl SpanLike {
		match self {
			Self::Declaration(declaration) => declaration.span().owned(),
			Self::Tail(tail) => tail.span().owned(),
			Self::Expression(expression) => expression.span().owned(),
			Self::Error(span) => span.owned(),
		}
	}
}

impl ToCabin for Statement {
	fn to_cabin(&self) -> String {
		match self {
			Statement::Declaration(declaration) => todo!(),
			Statement::Tail(tail_statement) => todo!(),
			Statement::Expression(expression) => todo!(),
			Statement::Error(span) => todo!(),
		}
	}
}
