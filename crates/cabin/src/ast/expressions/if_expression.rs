use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::expressions::{Expression, block::Block},
	comptime::{Check as _, Evaluate, storage::Pointer},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType,
};

#[derive(Debug, Clone, Getters)]
pub struct IfExpression {
	condition: Pointer,
	body: Block,
	otherwise_ifs: Vec<OtherwiseIf>,
	otherwise: Option<Otherwise>,
	tokens: IfTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct IfTokens {
	keyword_if: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct OtherwiseIf {
	condition: Pointer,
	body: Block,
	tokens: OtherwiseIfTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct Otherwise {
	body: Block,
	tokens: OtherwiseTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct OtherwiseTokens {
	keyword_otherwise: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct OtherwiseIfTokens {
	keyword_otherwise: Token,
	keyword_if: Token,
}

impl TryParse for IfExpression {
	type Output = IfExpression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_if = tokens.pop(TokenType::KeywordIf, context)?;
		let condition = Expression::try_parse(tokens, context)?;
		let body = Block::try_parse(tokens, context, ScopeType::If)?;

		let mut otherwise_ifs = Vec::new();
		while tokens.next_is(TokenType::KeywordOtherwise) && tokens.get(1).is_some_and(|t| t.token_type == TokenType::KeywordIf) {
			let keyword_otherwise = tokens.pop(TokenType::KeywordOtherwise, context).unwrap();
			let otherwise_keyword_if = tokens.pop(TokenType::KeywordIf, context).unwrap();
			let otherwise_condition = Expression::try_parse(tokens, context)?;
			let otherwise_body = Block::try_parse(tokens, context, ScopeType::OtherwiseIf)?;
			otherwise_ifs.push(OtherwiseIf {
				condition: otherwise_condition,
				body: otherwise_body,
				tokens: OtherwiseIfTokens {
					keyword_otherwise,
					keyword_if: otherwise_keyword_if,
				},
			});
		}

		let otherwise = if tokens.next_is(TokenType::KeywordOtherwise) {
			let keyword_otherwise = tokens.pop(TokenType::KeywordOtherwise, context).unwrap();
			let otherwise_body = Block::try_parse(tokens, context, ScopeType::Otherwise)?;
			Some(Otherwise {
				body: otherwise_body,
				tokens: OtherwiseTokens { keyword_otherwise },
			})
		} else {
			None
		};

		Ok(IfExpression {
			condition,
			body,
			otherwise_ifs,
			otherwise,
			tokens: IfTokens { keyword_if },
		})
	}
}

impl Evaluate for IfExpression {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		// evaluate condition
		let condition = self.condition.evaluate(context);
		let cabin_true = context.get_true();
		let condition_is_true = condition == cabin_true;

		// static analysis
		self.body.check(context);
		if let Some(otherwise) = self.otherwise.as_ref() {
			otherwise.body.check(context);
		}
		for otherwise_if in &self.otherwise_ifs {
			otherwise_if.body.check(context);
		}

		// main if
		if condition_is_true {
			return self.body.evaluate(context);
		}

		// otherwise ifs
		let mut otherwise_ifs = Vec::new();
		for otherwise_if in self.otherwise_ifs {
			let otherwise_condition = otherwise_if.condition.evaluate(context);
			let otherwise_condition_is_true = otherwise_condition == cabin_true;
			if otherwise_condition_is_true {
				return otherwise_if.body.evaluate(context);
			}

			otherwise_ifs.push(OtherwiseIf {
				condition: otherwise_condition,
				body: otherwise_if.body,
				tokens: otherwise_if.tokens,
			});
		}

		// otherwise
		let evaluated_otherwise = if let Some(otherwise) = self.otherwise {
			if !condition_is_true {
				return otherwise.body.evaluate(context);
			}
			otherwise.body.check(context);
			Some(otherwise)
		} else {
			None
		};

		// Non-literal: Return as an if-expression
		Expression::If(IfExpression {
			condition,
			body: self.body,
			otherwise_ifs,
			otherwise: evaluated_otherwise,
			tokens: self.tokens,
		})
		.store(context)
	}
}

impl Spanned for IfExpression {
	fn span(&self) -> impl SpanLike {
		let start = self.tokens().keyword_if().span();

		if let Some(otherwise) = &self.otherwise {
			return start.to(otherwise.body.span());
		}

		if let Some(last_otherwise_if) = self.otherwise_ifs.last() {
			return start.to(last_otherwise_if.body.span());
		}

		start.to(self.body.span())
	}
}
