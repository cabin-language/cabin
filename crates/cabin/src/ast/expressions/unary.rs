use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	ast::expressions::{Spanned, UnevaluatedExpression},
	comptime::{Evaluate, storage::ExpressionRef},
	lexer::Token,
};

/// A unary operator. These are types of operators that take a single expression and operate on it.
#[derive(Debug, Clone)]
pub enum UnaryOperator {
	Not,
	Assert,
}

impl UnaryOperator {
	pub const fn placement(&self) -> UnaryOperatorPlacement {
		match self {
			UnaryOperator::Not => UnaryOperatorPlacement::Prefix,
			UnaryOperator::Assert => UnaryOperatorPlacement::Postfix,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum UnaryOperatorPlacement {
	Prefix,
	Postfix,
}

#[derive(Debug, Clone, Getters)]
pub struct UnaryExpression {
	operator: UnaryOperator,
	expression: UnevaluatedExpression,
	tokens: UnaryTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct UnaryTokens {
	operator: Token,
}

impl Evaluate for UnaryExpression {
	type Output = ExpressionRef;

	fn evaluate(self, _context: &mut Context) -> Self::Output {
		match self.operator {
			UnaryOperator::Not => todo!(),
			UnaryOperator::Assert => todo!(),
		}
	}
}

impl Spanned for UnaryExpression {
	fn span(&self) -> impl SpanLike {
		match self.operator.placement() {
			UnaryOperatorPlacement::Prefix => self.tokens().operator().span().to(self.expression().span()),
			UnaryOperatorPlacement::Postfix => self.expression().span().to(self.tokens().operator().span()),
		}
	}
}
