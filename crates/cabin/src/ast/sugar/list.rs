use std::ops::Deref;

use crate::{
	api::context::Context,
	ast::expressions::{new_literal::Literal, Expression},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
	},
	diagnostics::Diagnostic,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TryParse},
};

#[derive(Debug, Clone)]
pub struct List(Vec<ExpressionPointer>);

impl TryParse for List {
	type Output = List;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut list = Vec::new();
		let _end = parse_list!(tokens, context, ListType::Bracketed, { list.push(Expression::parse(tokens, context)) }).span;
		Ok(List(list))
	}
}

impl CompileTime for List {
	type Output = Expression;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let items = self.0.into_iter().map(|item| item.evaluate_at_compile_time(context)).collect::<Vec<_>>();
		if items.iter().all(|item| item.is_literal(context)) {
			Expression::Literal(Literal::List(LiteralList(items.into_iter().map(|item| item.as_literal(context)).collect())))
		} else {
			Expression::List(List(items))
		}
	}
}

impl Deref for List {
	type Target = Vec<ExpressionPointer>;

	fn deref(&self) -> &Self::Target {
		&self.0
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
