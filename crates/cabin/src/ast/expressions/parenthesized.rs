use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::expressions::Expression,
	comptime::{Evaluate, storage::Pointer},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct ParenthesizedExpression {
	expression: Pointer,
	tokens: ParenthesizedTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ParenthesizedTokens {
	left_parenthesis: Token,
	right_parenthesis: Token,
}

impl TryParse for ParenthesizedExpression {
	type Output = Self;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let left_parenthesis = tokens.pop(TokenType::LeftParenthesis, context)?;
		let expression = Expression::try_parse(tokens, context)?;
		let right_parenthesis = tokens.pop(TokenType::RightParenthesis, context)?;

		Ok(ParenthesizedExpression {
			expression,
			tokens: ParenthesizedTokens {
				left_parenthesis,
				right_parenthesis,
			},
		})
	}
}

impl Spanned for ParenthesizedExpression {
	fn span(&self) -> impl SpanLike {
		self.tokens().left_parenthesis().span().to(self.tokens().right_parenthesis().span())
	}
}

impl ToCabin for ParenthesizedExpression {
	fn to_cabin(&self) -> String {
		format!("({})", self.expression.to_cabin())
	}
}

impl Evaluate for ParenthesizedExpression {
	type Output = ParenthesizedExpression;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		ParenthesizedExpression {
			expression: self.expression.evaluate(context),
			tokens: self.tokens,
		}
	}
}
