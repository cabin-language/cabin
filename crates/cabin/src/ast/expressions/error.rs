use derive_getters::Getters;

use crate::{SpanLike, Spanned, lexer::Token};

#[derive(Debug, Clone, Getters)]
pub struct ErrorExpression {
	tokens: ErrorExpressionTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ErrorExpressionTokens {
	tokens: Vec<Token>,
}

impl ErrorExpression {
	pub const fn new(tokens: Vec<Token>) -> ErrorExpression {
		ErrorExpression {
			tokens: ErrorExpressionTokens { tokens },
		}
	}
}

impl Spanned for ErrorExpression {
	fn span(&self) -> impl SpanLike {
		self.tokens().tokens().first().unwrap().span().to(self.tokens().tokens().last().unwrap().span())
	}
}
