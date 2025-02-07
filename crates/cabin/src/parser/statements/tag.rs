use std::{fmt::Debug, ops::Deref};

use crate::{
	api::traits::TryAs as _,
	comptime::{memory::VirtualPointer, CompileTime},
	debug_log,
	debug_start,
	mapped_err,
	parse_list,
	parser::{
		expressions::{literal::CompilerWarning, Expression},
		ListType,
		Parse,
		TokenQueue,
	},
};

#[derive(Clone, Default)]
pub struct TagList {
	pub values: Vec<Expression>,
}

impl Parse for TagList {
	type Output = TagList;

	fn parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Error> {
		let mut tags = Vec::new();
		let _ = parse_list!(tokens, ListType::Tag, {
			tags.push(Expression::parse(tokens)?);
		}); // TODO: Probably span this maybe?
		Ok(TagList { values: tags })
	}
}

impl CompileTime for TagList {
	type Output = TagList;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let debug_section = debug_start!("{} a {}", "Compile-Time Evaluating".green().bold(), "tag list".cyan());
		let mut values = Vec::new();
		for value in self.values {
			let evaluated = value.evaluate_at_compile_time().map_err(mapped_err! {
				while = "evaluating a tag at compile-time",
			})?;
			debug_log!("Evaluated tag into {}", evaluated.kind_name().cyan());
			values.push(evaluated);
		}
		debug_section.finish();
		Ok(TagList { values })
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
