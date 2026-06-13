use derive_getters::Getters;

use crate::{
	Context,
	Span,
	Spanned,
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct Number {
	value: f64,
	tokens: NumberTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct NumberTokens {
	number: Token,
}

impl TryParse for Number {
	type Output = Number;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let number = tokens.pop(TokenType::Number, context)?;

		Ok(Number {
			value: number.value.parse().unwrap(),
			tokens: NumberTokens { number },
		})
	}
}

impl Spanned for Number {
	fn span(&self) -> Span {
		self.tokens.number.span
	}
}
