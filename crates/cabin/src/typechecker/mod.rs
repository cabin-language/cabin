use std::fmt::Write as _;

use crate::{Context, ast::expressions::literal::EvaluatedLiteral, comptime::memory::LiteralPointer};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Type {
	Literal(LiteralPointer),
}

pub trait Typed {
	fn get_type(&self, context: &mut Context) -> Type;
}

impl Type {
	pub fn is_assignable_to(&self, other: &Type, context: &mut Context) -> bool {
		let Type::Literal(source) = self;
		let Type::Literal(target) = other;

		if source == &LiteralPointer::ERROR || target == &LiteralPointer::ERROR {
			return true;
		}

		let any = context
			.scope
			.get_builtin("Any")
			.unwrap()
			.to_owned()
			.try_as_literal(context)
			.unwrap_or(LiteralPointer::ERROR);

		if target == &any {
			return true;
		}

		source == target
	}

	pub fn name(&self, context: &mut Context) -> String {
		match self {
			Type::Literal(literal) => match literal.evaluated_literal(context).to_owned() {
				EvaluatedLiteral::Group(group) => group.name.as_ref().map_or_else(|| "Unknown".to_owned(), |name| name.source_identifier().to_owned()),
				EvaluatedLiteral::Action(function) => {
					let mut result = "action".to_owned();
					if !function.compile_time_parameters().is_empty() {
						write!(
							result,
							"<{}>",
							function
								.compile_time_parameters()
								.iter()
								.map(|parameter| parameter.name().source_identifier().to_owned())
								.collect::<Vec<_>>()
								.join(", "),
						)
						.unwrap();
					}
					if !function.parameters().is_empty() {
						write!(
							result,
							"({})",
							function
								.parameters()
								.iter()
								.map(|parameter| format!("{}: {}", parameter.name().source_identifier(), parameter.parameter_type().name(context)))
								.collect::<Vec<_>>()
								.join(", "),
						)
						.unwrap();
					}
					if let Some(return_type) = function.return_type() {
						write!(result, ": {}", return_type.name(context)).unwrap();
					}
					result
				},
				_ => "Unknown".to_owned(),
			},
		}
	}
}

pub trait Check {
	fn is_valid(&self, context: &mut Context) -> bool;
}
