use crate::{ast::expressions::new_literal::Literal, comptime::memory::LiteralPointer, Context};

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

		let anything = context.scope_tree.get_builtin("Anything").unwrap().to_owned().as_literal(context);
		if target == &anything {
			return true;
		}

		if source == &LiteralPointer::ERROR || target == &LiteralPointer::ERROR {
			return true;
		}

		source == target
	}

	pub fn name(&self, context: &Context) -> String {
		match self {
			Type::Literal(literal) => match literal.get_literal(context) {
				Literal::Group(group) => group
					.name
					.as_ref()
					.map(|name| name.unmangled_name().to_owned())
					.clone()
					.unwrap_or("<anonymous group>".to_owned()),
				Literal::Object(_) => "<anonymous object>".to_owned(),
				Literal::Either(_) => "<anonymous either>".to_owned(),
				Literal::Extend(_) => "<anonymous extension>".to_owned(),
				Literal::FunctionDeclaration(_) => "<anonymous function declaration>".to_owned(),
				Literal::Number(_) => "<anonymous number>".to_owned(),
				Literal::List(_) => "<anonymous list>".to_owned(),
				Literal::String(_) => "<anonymous string>".to_owned(),
				Literal::ErrorLiteral(_) => "<anonymous error>".to_owned(),
			},
		}
	}
}

pub trait Check {
	fn is_valid(&self, context: &mut Context) -> bool;
}
