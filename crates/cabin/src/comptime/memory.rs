use std::{
	collections::{HashMap, hash_map},
	fmt::Debug,
};

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::{evaluated::EvaluatedExpression, object::Object},
};

#[derive(Debug, Clone)]
pub enum Storable {
	Object(Object),
	Expression(EvaluatedExpression),
	None,
}

impl Storable {
	pub const fn as_object(&self) -> Option<&Object> {
		match self {
			Self::Object(object) => Some(object),
			_ => None,
		}
	}

	pub const fn as_expression(&self) -> Option<&EvaluatedExpression> {
		match self {
			Self::Expression(expression) => Some(expression),
			_ => None,
		}
	}
}

impl Spanned for Storable {
	fn span(&self) -> Span {
		match self {
			Storable::Object(object) => object.span(),
			Storable::Expression(expression) => expression.span(),
			Storable::None => Span::none(),
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Pointer(usize, Span);

impl PartialEq for Pointer {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl Eq for Pointer {}

impl Pointer {
	/// Creates a new pointer to the same place in memory as `self`, but with the given source span.
	pub const fn with_span(&self, span: Span) -> Pointer {
		Pointer(self.0, span)
	}

	pub fn value(self, context: &Context) -> &Storable {
		context.memory.get(self)
	}

	pub fn kind_name(&self, context: &Context) -> &'static str {
		match self.value(context) {
			Storable::Object(_) => "object",
			Storable::Expression(expression) => expression.kind_name(),
			Storable::None => "uninitialized memory",
		}
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		match self.value(context) {
			Storable::Object(object) => object.is_fully_known(context),
			Storable::Expression(expression) => expression.is_fully_known(),
			Storable::None => false,
		}
	}

	/// Creates a new empty space in memory and returns a pointer to it.
	pub fn calloc(context: &mut Context) -> Pointer {
		let id = context.memory.next_unused_virtual_address();
		let _ = context.memory.memory.insert(id, Storable::None);
		Pointer(id, Span::none())
	}
}

impl Spanned for Pointer {
	fn span(&self) -> Span {
		self.1
	}
}

#[derive(Debug, Clone)]
pub struct VirtualMemory {
	memory: HashMap<usize, Storable>,
}

impl<'mem> IntoIterator for &'mem VirtualMemory {
	type IntoIter = hash_map::Iter<'mem, usize, Storable>;
	type Item = (&'mem usize, &'mem Storable);

	fn into_iter(self) -> Self::IntoIter {
		self.memory.iter()
	}
}

impl VirtualMemory {
	pub fn empty() -> VirtualMemory {
		VirtualMemory { memory: HashMap::new() }
	}

	pub fn store(&mut self, value: impl Into<Storable>) -> Pointer {
		let address = self.next_unused_virtual_address();
		let storable = value.into();
		let pointer = Pointer(address, storable.span());
		let _ = self.memory.insert(address, storable);
		pointer
	}

	pub fn calloc(&mut self) -> Pointer {
		let id = self.next_unused_virtual_address();
		let _ = self.memory.insert(id, Storable::None);
		Pointer(id, Span::none())
	}

	pub fn get(&self, address: Pointer) -> &Storable {
		self.memory.get(&address.0).unwrap()
	}

	fn next_unused_virtual_address(&self) -> usize {
		let mut next_unused_virtual_address = 1;
		while self.memory.contains_key(&next_unused_virtual_address) {
			next_unused_virtual_address += 1;
		}
		next_unused_virtual_address
	}
}
