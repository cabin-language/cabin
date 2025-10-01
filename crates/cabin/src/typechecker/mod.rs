use std::fmt::Write as _;

use crate::{ast::expressions::literal::EvaluatedLiteral, comptime::memory::LiteralPointer, io::Io, Context};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Type {
	Literal(LiteralPointer),
}

pub trait Typed {
	fn get_type<System: Io>(&self, context: &mut Context<System>) -> Type;
}

impl Type {
	pub(crate) fn is_assignable_to<System: Io>(&self, other: &Type, context: &mut Context<System>) -> bool {
		let Type::Literal(source) = self;
		let Type::Literal(target) = other;

		if source == &LiteralPointer::ERROR || target == &LiteralPointer::ERROR {
			return true;
		}

		let anything = context
			.scope_tree
			.get_builtin("Anything")
			.unwrap()
			.to_owned()
			.try_as_literal(context)
			.unwrap_or(LiteralPointer::ERROR);

		if target == &anything {
			return true;
		}

		source == target
	}

	pub fn name<System: Io>(&self, context: &mut Context<System>) -> String {
		match self {
			Type::Literal(literal) => match literal.evaluated_literal(context).to_owned() {
				EvaluatedLiteral::Group(group) => group.name.as_ref().map_or_else(|| "Unknown".to_owned(), |name| name.unmangled_name().to_owned()),
				EvaluatedLiteral::FunctionDeclaration(function) => {
					let mut result = "action".to_owned();
					if !function.compile_time_parameters().is_empty() {
						write!(
							result,
							"<{}>",
							function
								.compile_time_parameters()
								.iter()
								.map(|parameter| parameter.name().unmangled_name().to_owned())
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
								.map(|parameter| format!("{}: {}", parameter.name().unmangled_name(), parameter.parameter_type().name(context)))
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
	fn is_valid<System: Io>(&self, context: &mut Context<System>) -> bool;
}
