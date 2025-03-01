use crate::{comptime::memory::LiteralPointer, Context};

#[derive(Debug, Clone)]
pub enum Type {
	Literal(LiteralPointer),
}

pub(crate) trait Typed {
	fn get_type(&self, context: &mut Context) -> Type;
}

impl Type {
	pub(crate) fn is_assignable_to(&self, other: &Type) -> bool {
		let Type::Literal(source) = self;
		let Type::Literal(target) = other;

		source == target
	}
}

pub trait Check {
	fn is_valid(&self, context: &mut Context) -> bool;
}
