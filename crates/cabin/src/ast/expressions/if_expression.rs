use super::ExpressionOrPointer;
use crate::{
	Span,
	Spanned,
	api::{context::Context, scope::ScopeId},
	ast::expressions::{Expression, block::Block},
	comptime::{CompileTime, memory::ExpressionPointer},
	diagnostics::Diagnostic,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
};

#[derive(Debug, Clone)]
pub struct IfExpression {
	condition: ExpressionPointer,
	body: ExpressionPointer,
	else_body: Option<ExpressionPointer>,
	span: Span,
}

impl TryParse for IfExpression {
	type Output = IfExpression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
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
			body: Expression::Block(body).store_in_memory(context),
			else_body: else_body.map(|else_body| Expression::Block(else_body).store_in_memory(context)),
			span: start.to(end),
		})
	}
}

impl CompileTime for IfExpression {
	type Output = ExpressionOrPointer;

	#[allow(clippy::almost_swapped, reason = "False positive; context.side_effects is reassigned")]
	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		// Check condition
		let condition = self.condition.evaluate_at_compile_time(context);
		let cabin_true = context.scope.get_variable_from_id("true", ScopeId::global()).unwrap();
		let condition_is_true = condition == cabin_true;

		// Evaluate body
		let snapshot = context.snapshot();
		let body = self.body.evaluate_at_compile_time(context);

		// IF the condition was false, then the if body shouldnt have side effects--
		// so revert to the previous snapshot
		if !condition_is_true {
			context.roll_back(snapshot);
		}

		// Evaluate else body
		let else_snapshot = context.snapshot();
		let else_body = self.else_body.map(|else_body| else_body.evaluate_at_compile_time(context));

		// IF the condition was true, then the else body shouldnt have side effects--
		// so revert to the previous snapshot
		if condition_is_true {
			context.roll_back(else_snapshot);
		}

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
	fn to_c(&self, context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		Ok(format!("if ({}) {}", self.condition.to_c(context, None)?, self.body.to_c(context, None)?))
	}
}

impl Spanned for IfExpression {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}
