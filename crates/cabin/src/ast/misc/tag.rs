use std::{fmt::Debug, ops::Deref};

use crate::{
	api::context::Context,
	ast::expressions::Expression,
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::Diagnostic,
	io::Io,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TryParse},
};

#[derive(Clone, Default)]
pub struct TagList {
	pub values: Vec<ExpressionPointer>,
}

impl TagList {
	pub const fn empty() -> TagList {
		TagList { values: Vec::new() }
	}
}

impl TryParse for TagList {
	type Output = TagList;

	fn try_parse<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>) -> Result<Self::Output, Diagnostic> {
		let mut tags = Vec::new();
		let _ = parse_list!(tokens, context, ListType::Tag, {
			tags.push(Expression::parse(tokens, context));
		});
		Ok(TagList { values: tags })
	}
}

impl CompileTime for TagList {
	type Output = TagList;

	fn evaluate_at_compile_time<System: Io>(self, context: &mut Context<System>) -> Self::Output {
		let mut values = Vec::new();
		for value in self.values {
			let evaluated = value.evaluate_at_compile_time(context);
			values.push(evaluated);
		}
		TagList { values }
	}
}

impl Deref for TagList {
	type Target = Vec<ExpressionPointer>;

	fn deref(&self) -> &Self::Target {
		&self.values
	}
}

impl From<Vec<ExpressionPointer>> for TagList {
	fn from(values: Vec<ExpressionPointer>) -> Self {
		Self { values }
	}
}

impl Debug for TagList {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			format!("#[{}]", self.values.iter().map(|value| format!("{value:?}")).collect::<Vec<_>>().join(", "))
				.replace("\n", " ")
				.replace("\t", "")
		)
	}
}
