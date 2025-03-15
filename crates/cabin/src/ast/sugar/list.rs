use std::ops::Deref;

use crate::{
	api::context::Context,
	ast::expressions::{literal::EvaluatedLiteral, Expression},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
	},
	diagnostics::Diagnostic,
	io::{IoReader, IoWriter},
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TryParse},
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct List {
	span: Span,
	elements: Vec<ExpressionPointer>,
}

impl TryParse for List {
	type Output = List;

	fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Result<Self::Output, Diagnostic> {
		let mut list = Vec::new();
		let end = parse_list!(tokens, context, ListType::Bracketed, { list.push(Expression::parse(tokens, context)) }).span;

		Ok(List {
			elements: list,
			span: Span::unknown(),
		})
	}
}

impl CompileTime for List {
	type Output = Expression;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		let items = self.elements.into_iter().map(|item| item.evaluate_at_compile_time(context)).collect::<Vec<_>>();
		if items.iter().all(|item| item.is_literal(context)) {
			Expression::EvaluatedLiteral(EvaluatedLiteral::List(LiteralList(items.into_iter().map(|item| item.as_literal(context)).collect())))
		} else {
			Expression::List(List { elements: items, span: self.span })
		}
	}
}

impl Spanned for List {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
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
	pub(crate) fn empty() -> LiteralList {
		LiteralList(Vec::new())
	}
}

impl Deref for LiteralList {
	type Target = Vec<LiteralPointer>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
