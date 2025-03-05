use crate::{ast::expressions::new_literal::EvaluatedLiteral, comptime::memory::LiteralPointer, Context};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Type {
	Literal(LiteralPointer),
}

pub trait Typed {
	fn get_type(&self, context: &mut Context) -> Type;
}

impl Type {
	pub(crate) fn is_assignable_to(&self, other: &Type, context: &mut Context) -> bool {
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

	pub fn name(&self, context: &Context) -> String {
		match self {
			Type::Literal(literal) => match literal.get_literal(context) {
				EvaluatedLiteral::Group(group) => group.name.as_ref().map(|name| name.unmangled_name().to_owned()).clone().unwrap_or("Unknown".to_owned()),
				EvaluatedLiteral::FunctionDeclaration(function) => {
					let mut result = "action".to_owned();
					if !function.compile_time_parameters().is_empty() {
						result += &format!(
							"<{}>",
							function
								.compile_time_parameters()
								.iter()
								.map(|parameter| parameter.name().unmangled_name().to_owned())
								.collect::<Vec<_>>()
								.join(", "),
						);
					}
					if !function.parameters().is_empty() {
						result += &format!(
							"({})",
							function
								.parameters()
								.iter()
								.map(|parameter| format!("{}: {}", parameter.name().unmangled_name(), parameter.parameter_type().name(context)))
								.collect::<Vec<_>>()
								.join(", "),
						);
					}
					if let Some(return_type) = function.return_type() {
						result += &format!(": {}", return_type.name(context));
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
