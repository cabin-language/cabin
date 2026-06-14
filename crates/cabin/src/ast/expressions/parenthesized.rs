use derive_getters::Getters;

use crate::{
	Context,
	Spanned,
	ast::expressions::Expression,
	comptime::{Evaluate, memory::Pointer},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct ParenthesizedExpression {
	expression: Expression,
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
	fn span(&self) -> crate::Span {
		self.tokens.left_parenthesis.span.to(self.tokens.right_parenthesis.span)
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedParenthesizedExpression {
	expression: Pointer,
	tokens: ParenthesizedTokens,
}

impl Evaluate for ParenthesizedExpression {
	type Output = EvaluatedParenthesizedExpression;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedParenthesizedExpression {
			expression: self.expression.evaluate(context),
			tokens: self.tokens,
		}
	}
}

impl Spanned for EvaluatedParenthesizedExpression {
	fn span(&self) -> crate::Span {
		self.tokens.left_parenthesis.span.to(self.tokens.right_parenthesis.span)
	}
}
