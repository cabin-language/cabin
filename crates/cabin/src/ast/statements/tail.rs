use derive_getters::Getters;

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, identifier::Identifier},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct TailStatement {
	label: Identifier,
	value: Expression,
	tokens: TailStatementTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct TailStatementTokens {
	is: Token,
}

impl TryParse for TailStatement {
	type Output = TailStatement;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let label = Identifier::try_parse(tokens, context)?;
		let is = tokens.pop(TokenType::KeywordIs, context)?;
		let value = Expression::try_parse(tokens, context)?;

		Ok(TailStatement {
			label,
			value,
			tokens: TailStatementTokens { is },
		})
	}
}

impl Evaluate for TailStatement {
	type Output = EvaluatedTailStatement;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedTailStatement {
			label: self.label,
			value: self.value.evaluate(context),
			tokens: self.tokens,
		}
	}
}

impl Spanned for TailStatement {
	fn span(&self) -> Span {
		self.label.span().to(self.value.span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedTailStatement {
	label: Identifier,
	value: Pointer,
	tokens: TailStatementTokens,
}

impl Spanned for EvaluatedTailStatement {
	fn span(&self) -> Span {
		self.label.span().to(self.value.span())
	}
}
