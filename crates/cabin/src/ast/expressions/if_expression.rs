use super::ExpressionOrPointer;
use crate::{
	api::{context::Context, scope::ScopeId},
	ast::expressions::{block::Block, Expression},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::Diagnostic,
	io::Io,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct IfExpression {
	condition: ExpressionPointer,
	body: Block,
	else_body: Option<Block>,
	span: Span,
}

impl TryParse for IfExpression {
	type Output = IfExpression;

	fn try_parse<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordIf, context)?.span;
		let condition = Expression::parse(tokens, context);
		let body = Block::try_parse(tokens, context)?;
		let mut end = body.span(context);
		let else_body = if tokens.next_is(TokenType::KeywordOtherwise) {
			let _ = tokens.pop(TokenType::KeywordOtherwise, context).unwrap();
			let else_body = Block::try_parse(tokens, context)?;
			end = else_body.span(context);
			Some(else_body)
		} else {
			None
		};

		Ok(IfExpression {
			condition,
			body,
			else_body,
			span: start.to(end),
		})
	}
}

impl CompileTime for IfExpression {
	type Output = ExpressionOrPointer;

	#[allow(clippy::almost_swapped, reason = "False positive; context.side_effects is reassigned")]
	fn evaluate_at_compile_time<System: Io>(self, context: &mut Context<System>) -> Self::Output {
		// Check condition
		let condition = self.condition.evaluate_at_compile_time(context);
		let cabin_true = context.scope_tree.get_variable_from_id("true", ScopeId::global()).unwrap();
		let condition_is_true = condition == cabin_true;

		// Evaluate body
		let had_side_effects = context.side_effects;
		context.side_effects = had_side_effects && condition_is_true;
		let body = self.body.evaluate_at_compile_time(context);
		context.side_effects = had_side_effects;

		// Evaluate else body
		let had_side_effects_2 = context.side_effects;
		context.side_effects = had_side_effects_2 && !condition_is_true;
		let else_body = self.else_body.map(|else_body| else_body.evaluate_at_compile_time(context));
		context.side_effects = had_side_effects_2;

		// Fully evaluated: return the value (only if true)
		if condition_is_true {
			todo!()
		}
		// Else body
		else if let Some(_else_body) = &else_body {
			todo!()
		}

		// Non-literal: Return as an if-expression
		ExpressionOrPointer::Expression(Expression::If(IfExpression {
			condition,
			body,
			else_body,
			span: self.span,
		}))
	}
}

impl TranspileToC for IfExpression {
	fn to_c<System: Io>(&self, context: &mut Context<System>, _output: Option<String>) -> Result<String, TranspileError> {
		Ok(format!("if ({}) {}", self.condition.to_c(context, None)?, self.body.to_c(context, None)?))
	}
}

impl Spanned for IfExpression {
	fn span<System: Io>(&self, _context: &Context<System>) -> Span {
		self.span
	}
}
