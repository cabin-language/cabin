use std::ops::Deref;

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, literal::EvaluatedLiteral},
	comptime::{
		CompileTime,
		memory::{ExpressionPointer, LiteralPointer},
	},
	diagnostics::Diagnostic,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality, TryParse},
};

#[derive(Debug, Clone)]
pub struct List {
	span: Span,
	elements: Vec<ExpressionPointer>,
}

impl TryParse for List {
	type Output = List;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut list = Vec::new();
		let start = tokens.current_position().unwrap();
		let end = parse_list!(tokens, context, ListType::Bracketed, { list.push(Expression::parse(tokens, context)) }).span;

		Ok(List {
			elements: list,
			span: start.to(end),
		})
	}
}

impl CompileTime for List {
	type Output = Expression;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let items = self.elements.into_iter().map(|item| item.evaluate_at_compile_time(context)).collect::<Vec<_>>();
		if items.iter().all(|item| item.is_literal(context)) {
			Expression::EvaluatedLiteral(EvaluatedLiteral::List(LiteralList(items.into_iter().map(|item| item.as_literal(context)).collect())))
		} else {
			Expression::List(List { elements: items, span: self.span })
		}
	}
}

impl Spanned for List {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl Deref for List {
	type Target = Vec<ExpressionPointer>;

	fn deref(&self) -> &Self::Target {
		&self.elements
	}
}

#[derive(Debug, Clone)]
pub struct LiteralList(Vec<LiteralPointer>);

impl LiteralList {
	pub fn empty() -> LiteralList {
		LiteralList(Vec::new())
	}
}

impl Deref for LiteralList {
	type Target = Vec<LiteralPointer>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
