use std::fmt::Write as _;

use super::Spanned;
use crate::{
	api::{context::context, scope::ScopeId},
	comptime::CompileTime,
	lexer::{Span, TokenType},
	mapped_err,
	parser::{
		expressions::{block::Block, Expression},
		Parse,
		TokenQueue,
		TokenQueueFunctionality,
	},
	transpiler::TranspileToC,
};

#[derive(Debug, Clone)]
pub struct IfExpression {
	condition: Box<Expression>,
	body: Box<Expression>,
	else_body: Option<Box<Expression>>,
	span: Span,
	inner_scope_id: ScopeId,
}

impl Parse for IfExpression {
	type Output = IfExpression;

	fn parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Error> {
		let start = tokens.pop(TokenType::KeywordIf)?.span;
		let condition = Box::new(Expression::parse(tokens)?);
		let body = Block::parse(tokens)?;
		let mut end = body.span();
		let else_body = if tokens.next_is(TokenType::KeywordOtherwise) {
			let _ = tokens.pop(TokenType::KeywordOtherwise).unwrap();
			let else_body = Expression::Block(Block::parse(tokens)?);
			end = else_body.span();
			Some(Box::new(else_body))
		} else {
			None
		};
		Ok(IfExpression {
			condition,
			inner_scope_id: body.inner_scope_id(),
			body: Box::new(Expression::Block(body)),
			else_body,
			span: start.to(end),
		})
	}
}

impl CompileTime for IfExpression {
	type Output = Expression;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		// Check condition
		let condition = self.condition.evaluate_at_compile_time().map_err(mapped_err! {
			while = "evaluating the condition of an if-expression at compile-time",
		})?;
		let condition_is_true = condition.is_true();

		// Evaluate body
		context().toggle_side_effects(condition_is_true);
		let body = self
			.body
			.evaluate_at_compile_time()
			.map_err(mapped_err! { while = "evaluating the body of an if-expression at compile-time", })?;
		context().untoggle_side_effects();

		// Evaluate else body
		context().toggle_side_effects(!condition_is_true);
		let else_body = self
			.else_body
			.map(|else_body| {
				anyhow::Ok(Box::new(else_body.evaluate_at_compile_time().map_err(mapped_err! {
					while = "evaluating the otherwise-body of an if-expression at compile-time",
				})?))
			})
			.transpose()?;
		context().untoggle_side_effects();

		// Fully evaluated: return the value (only if true)
		if condition_is_true {
			if let Ok(literal) = body.try_clone_pointer() {
				return Ok(literal);
			}
		} else if let Some(else_body) = &else_body {
			if let Ok(literal) = else_body.try_clone_pointer() {
				return Ok(literal);
			}
		}

		// Non-literal: Return as an if-expression
		Ok(Expression::If(IfExpression {
			condition: Box::new(condition),
			body: Box::new(body),
			else_body,
			span: self.span,
			inner_scope_id: self.inner_scope_id,
		}))
	}
}

impl TranspileToC for IfExpression {
	fn to_c(&self) -> anyhow::Result<String> {
		let mut builder = format!("({}) ? (", self.condition.to_c()?);
		for line in self.body.to_c()?.lines() {
			write!(builder, "\n\t{line}").unwrap();
		}
		builder += "\n) : (";

		if let Some(else_body) = &self.else_body {
			for line in else_body.to_c()?.lines() {
				write!(builder, "\n\t{line}").unwrap();
			}
		} else {
			write!(builder, "\nNULL").unwrap();
		}

		builder += "\n) ";

		Ok(builder)
	}
}

impl Spanned for IfExpression {
	fn span(&self) -> Span {
		self.span
	}
}

impl IfExpression {
	pub const fn inner_scope_id(&self) -> ScopeId {
		self.inner_scope_id
	}
}
