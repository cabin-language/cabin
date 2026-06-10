use crate::{
	Context,
	Span,
	Spanned,
	ast::expressions::{Expression, block::Block},
	comptime::{CompileTime, memory::ExpressionPointer},
	diagnostics::Diagnostic,
	lexer::TokenType,
	parser::{Parse, TokenQueue, TokenQueueFunctionality, TryParse},
	scope::ScopeType::{self},
};

pub struct WhileLoop {
	pub condition: ExpressionPointer,
	pub body: Block,
	pub span: Span,
}

impl TryParse for WhileLoop {
	type Output = WhileLoop;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordWhile, context)?.span;
		let expression = Expression::parse(tokens, context);
		let body = Block::parse_with_scope_type(tokens, context, ScopeType::While)?;
		Ok(WhileLoop {
			condition: expression,
			span: start.to(body.span(context)),
			body,
		})
	}
}

impl CompileTime for WhileLoop {
	type Output = ExpressionPointer;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		loop {
			let condition = self.condition.clone().evaluate_at_compile_time(context);

			// loop can be run at compile-time
			if condition.is_literal(context) {
				// loop is done
				if condition == context.get_false() {
					return context.none();
				}
			}
			// loop cant be run at compile-time
			else {
				return context.none();
			}
		}
	}
}
