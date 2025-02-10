use std::{fmt::Debug, ops::Deref};

use crate::{
	api::traits::TryAs as _,
	comptime::{memory::VirtualPointer, CompileTime},
	diagnostics::Diagnostic,
	parse_list,
	parser::{
		expressions::{literal::CompilerWarning, Expression},
		ListType,
		Parse as _,
		TokenQueue,
		TryParse,
	},
};

#[derive(Clone, Default)]
pub struct TagList {
	pub values: Vec<Expression>,
}

impl TryParse for TagList {
	type Output = TagList;

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, Diagnostic> {
		let mut tags = Vec::new();
		let _ = parse_list!(tokens, ListType::Tag, {
			tags.push(Expression::parse(tokens));
		}); // TODO: Probably span this maybe?
		Ok(TagList { values: tags })
	}
}

impl CompileTime for TagList {
	type Output = TagList;

	fn evaluate_at_compile_time(self) -> Self::Output {
		let mut values = Vec::new();
		for value in self.values {
			let evaluated = value.evaluate_at_compile_time();
			values.push(evaluated);
		}
		TagList { values }
	}
}

impl Deref for TagList {
	type Target = Vec<Expression>;

	fn deref(&self) -> &Self::Target {
		&self.values
	}
}

impl From<Vec<Expression>> for TagList {
	fn from(values: Vec<Expression>) -> Self {
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

impl TagList {
	pub fn suppresses_warning(&self, warning: CompilerWarning) -> bool {
		if self.is_empty() {
			return false;
		}

		self.iter().any(|tag| {
			tag.try_as::<VirtualPointer>()
				.map(|pointer| {
					let literal = pointer.virtual_deref();
					if literal.type_name() == &"WarningSuppressor".into() {
						let value = literal.get_field_literal("warning").unwrap();
						return value.is_warning(warning);
					}
					false
				})
				.unwrap_or(false)
		})
	}
}
