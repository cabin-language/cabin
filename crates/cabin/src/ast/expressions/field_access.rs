use super::ExpressionOrPointer;
use crate::{
	api::{context::Context, scope::ScopeId},
	ast::expressions::{name::Name, operators::PrimaryExpression, Expression},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::Diagnostic,
	lexer::TokenType,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::{TranspileError, TranspileToC},
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct FieldAccess {
	left: ExpressionPointer,
	right: Name,
	scope_id: ScopeId,
	span: Span,
}

impl TryParse for FieldAccess {
	type Output = ExpressionPointer;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut expression = PrimaryExpression::try_parse(tokens, context)?;
		let start = expression.span(context);
		while tokens.next_is(TokenType::Dot, context) {
			let _ = tokens.pop(TokenType::Dot, context)?;
			let right = Name::try_parse(tokens, context)?;
			let end = right.span(context);
			expression = Expression::FieldAccess(Self {
				left: expression,
				right,
				scope_id: context.scope_tree.unique_id(),
				span: start.to(end),
			})
			.store_in_memory(context);
		}

		Ok(expression)
	}
}

impl CompileTime for FieldAccess {
	type Output = ExpressionOrPointer;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let left_evaluated = self.left.evaluate_at_compile_time(context);

		// Resolvable at compile-time
		if let Ok(pointer) = left_evaluated.try_as_literal(context) {
			let literal = pointer.literal(context).to_owned();
			let field_value = literal.dot(&self.right, context);
			ExpressionOrPointer::Pointer(field_value)
		}
		// Not resolvable at compile-time - return the original expression
		else {
			ExpressionOrPointer::Expression(Expression::FieldAccess(FieldAccess {
				left: left_evaluated,
				right: self.right,
				scope_id: self.scope_id,
				span: self.span,
			}))
		}
	}
}

impl TranspileToC for FieldAccess {
	fn to_c(&self, context: &mut Context, output: Option<String>) -> Result<String, TranspileError> {
		Ok(format!(
			"void* left;{}{}->{};",
			self.left.to_c(context, Some("left".to_owned()))?,
			if let Some(name) = output { format!("{name} = ") } else { String::new() },
			self.right.to_c(context, None)?
		))
	}
}

impl Spanned for FieldAccess {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl FieldAccess {
	pub(crate) fn new(left: ExpressionPointer, right: Name, scope_id: ScopeId, span: Span) -> FieldAccess {
		FieldAccess { left, right, scope_id, span }
	}
}

pub trait Dot {
	fn dot(&self, name: &Name, context: &mut Context) -> ExpressionPointer;
}
