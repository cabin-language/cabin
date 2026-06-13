use derive_getters::Getters;

use crate::{
	Context,
	Span,
	ast::expressions::{Expression, Spanned, evaluated::EvaluatedExpression},
	comptime::Evaluate,
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct RunExpression {
	expression: Expression,
	tokens: RunExpressionTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct RunExpressionTokens {
	keyword_run: Token,
	left_parenthesis: Token,
	right_parenthesis: Token,
}

impl TryParse for RunExpression {
	type Output = RunExpression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_run = tokens.pop(TokenType::KeywordRun, context)?;
		let left_parenthesis = tokens.pop(TokenType::LeftParenthesis, context)?;
		let expression = Expression::try_parse(tokens, context)?;
		let right_parenthesis = tokens.pop(TokenType::RightParenthesis, context)?;

		Ok(RunExpression {
			expression,
			tokens: RunExpressionTokens {
				keyword_run,
				left_parenthesis,
				right_parenthesis,
			},
		})
	}
}

impl Evaluate for RunExpression {
	type Output = EvaluatedRunExpression;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedRunExpression {
			expression: self.expression.evaluate_subexpressions(context),
			tokens: self.tokens,
		}
	}
}

impl ToCabin for RunExpression {
	fn to_cabin(&self) -> String {
		format!("run({})", self.expression.to_cabin())
	}
}

impl Spanned for RunExpression {
	fn span(&self) -> Span {
		self.tokens.keyword_run.span.to(self.tokens.right_parenthesis.span)
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedRunExpression {
	expression: EvaluatedExpression,
	tokens: RunExpressionTokens,
}

pub trait Runtimeable {
	type Output;
	fn evaluate_subexpressions(self, context: &mut Context) -> Self::Output;
}

impl Spanned for EvaluatedRunExpression {
	fn span(&self) -> Span {
		self.tokens.keyword_run.span.to(self.tokens.right_parenthesis.span)
	}
}
