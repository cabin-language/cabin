use std::{
	collections::{HashMap, hash_map},
	fmt::Debug,
	marker::PhantomData,
};

use crate::{
	Context,
	Span,
	SpanLike,
	Spanned,
	ast::expressions::{
		Expression,
		field_access::{FieldAccess, FieldAccessTokens},
		identifier::Identifier,
		list::List,
		number::Number,
		object::Object,
		run::Runtime,
		string::Text,
	},
	comptime::{Dot, Evaluate},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TryParse},
	traits::TryAs as _,
	transpiler::ToCabin,
};

#[derive(Debug, Clone)]
pub enum Storable {
	Expression(Expression),
	Pointer(Pointer),
	Object(Object),
	Uninitialized,
}

impl Storable {
	pub fn is_object(&self) -> bool {
		matches!(self, Storable::Object(_))
	}

	pub fn as_expression(&self) -> Option<&Expression> {
		match self {
			Storable::Expression(expression) => Some(expression),
			_ => None,
		}
	}

	pub fn as_expression_mut(&mut self) -> Option<&mut Expression> {
		match self {
			Storable::Expression(expression) => Some(expression),
			_ => None,
		}
	}

	pub fn as_object(&self) -> Option<&Object> {
		match self {
			Storable::Object(object) => Some(object),
			_ => None,
		}
	}

	pub fn as_object_mut(&mut self) -> Option<&mut Object> {
		match self {
			Storable::Object(object) => Some(object),
			_ => None,
		}
	}
}

impl Spanned for Storable {
	fn span(&self) -> impl SpanLike {
		match self {
			Storable::Expression(expression) => expression.span().owned(),
			Storable::Object(object) => object.span().owned(),
			Storable::Uninitialized => Span::none(),
			Storable::Pointer(pointer) => pointer.span().owned(),
		}
	}
}

impl From<Object> for Storable {
	fn from(expression: Object) -> Self {
		Storable::Object(expression)
	}
}

impl<'ctx> TryFrom<&'ctx Storable> for &'ctx Object {
	type Error = ();

	fn try_from(value: &'ctx Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Object(object) => Ok(object),
			_ => Err(()),
		}
	}
}

impl<'ctx> TryFrom<&'ctx mut Storable> for &'ctx mut Object {
	type Error = ();

	fn try_from(value: &'ctx mut Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Object(object) => Ok(object),
			_ => Err(()),
		}
	}
}

impl From<Expression> for Storable {
	fn from(expression: Expression) -> Self {
		Storable::Expression(expression.into())
	}
}

impl<'ctx> TryFrom<&'ctx Storable> for &'ctx Expression {
	type Error = ();

	fn try_from(value: &'ctx Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Expression(expression) => Ok(expression),
			_ => Err(()),
		}
	}
}

impl<'ctx> TryFrom<&'ctx mut Storable> for &'ctx mut Expression {
	type Error = ();

	fn try_from(value: &'ctx mut Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Expression(expression) => Ok(expression),
			_ => Err(()),
		}
	}
}

impl ToCabin for Pointer {
	fn to_cabin(&self) -> String {
		todo!()
	}
}

#[derive(Debug, Clone)]
pub struct Pointer(usize, Span);

impl Pointer {
	pub fn value<'ctx>(&self, context: &'ctx Context) -> &'ctx Storable {
		let mut value = context.storage.get(self);
		while let Storable::Pointer(pointer) = value {
			value = pointer.value(context);
		}

		value
	}

	pub fn value_mut<'ctx>(&self, context: &'ctx mut Context) -> &'ctx mut Storable {
		context.storage.get_mut(self)
	}

	pub fn with_span(&self, span: Span) -> Self {
		Pointer(self.0, span)
	}

	pub fn is_object(&self, context: &Context) -> bool {
		self.value(context).is_object()
	}

	pub fn as_object<'ctx>(&self, context: &'ctx Context) -> Option<&'ctx Object> {
		self.value(context).as_object()
	}

	pub fn as_object_mut<'ctx>(&self, context: &'ctx mut Context) -> Option<&'ctx mut Object> {
		self.value_mut(context).as_object_mut()
	}

	pub fn as_expression<'ctx>(&self, context: &'ctx Context) -> Option<&'ctx Expression> {
		self.value(context).as_expression()
	}

	pub fn as_expression_mut<'ctx>(&self, context: &'ctx mut Context) -> Option<&'ctx mut Expression> {
		self.value_mut(context).as_expression_mut()
	}

	pub fn as_text_literal<'ctx>(&self, context: &'ctx Context) -> Option<&'ctx String> {
		self.as_expression(context)
			.and_then(|expression| expression.try_as::<Text>().ok())
			.and_then(|text| text.as_literal())
	}

	pub fn as_number_literal(&self, context: &Context) -> Option<f64> {
		self.as_expression(context)
			.and_then(|expression| expression.try_as::<Number>().ok())
			.map(|number| number.value)
	}

	pub fn as_list_literal<'ctx>(&self, context: &'ctx Context) -> Option<&'ctx List> {
		self.as_expression(context).and_then(|expression| expression.try_as::<List>().ok())
	}

	pub fn as_mut_list_literal<'ctx>(&self, context: &'ctx mut Context) -> Option<&'ctx mut List> {
		self.as_expression_mut(context).and_then(|expression| expression.try_mut::<List>().ok())
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		match self.value(context) {
			Storable::Expression(expression) => expression.is_fully_known(context),
			Storable::Object(object) => true,
			Storable::Uninitialized => false,
			Storable::Pointer(pointer) => pointer.is_fully_known(context),
		}
	}
}

impl Evaluate for Pointer {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let storable = context.storage.get(&self).clone();
		let evaluated = match storable {
			Storable::Expression(expression) => expression.evaluate(context),
			Storable::Uninitialized => return self,
			Storable::Object(_) => todo!(),
			Storable::Pointer(pointer) => pointer.evaluate(context),
		};
		context.storage.memory.insert(self.0, Storable::Pointer(evaluated));
		Pointer(self.0, self.1)
	}
}

impl Spanned for Pointer {
	fn span(&self) -> impl SpanLike {
		&self.1
	}
}

impl PartialEq for Pointer {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl Eq for Pointer {}

#[derive(Debug, Clone)]
pub struct Storage {
	memory: HashMap<usize, Storable>,
}

impl<'mem> IntoIterator for &'mem Storage {
	type IntoIter = hash_map::Iter<'mem, usize, Storable>;
	type Item = (&'mem usize, &'mem Storable);

	fn into_iter(self) -> Self::IntoIter {
		self.memory.iter()
	}
}

impl Storage {
	pub fn empty() -> Storage {
		Storage { memory: HashMap::new() }
	}

	pub fn store(&mut self, value: impl Into<Storable> + Spanned) -> Pointer {
		let address = self.next_unused_virtual_address();
		let pointer = Pointer(address, value.span().owned());
		let _ = self.memory.insert(address, value.into());
		pointer
	}

	pub fn calloc(&mut self) -> Pointer {
		let id = self.next_unused_virtual_address();
		let _ = self.memory.insert(id, Storable::Uninitialized);
		Pointer(id, Span::none())
	}

	pub fn get(&self, address: &Pointer) -> &Storable {
		self.memory.get(&address.0).unwrap().try_into().unwrap()
	}

	pub fn get_mut(&mut self, address: &Pointer) -> &mut Storable {
		self.memory.get_mut(&address.0).unwrap().try_into().unwrap()
	}

	fn next_unused_virtual_address(&self) -> usize {
		let mut next_unused_virtual_address = 1;
		while self.memory.contains_key(&next_unused_virtual_address) {
			next_unused_virtual_address += 1;
		}
		next_unused_virtual_address
	}
}

impl Runtime for Pointer {
	fn evaluate_subexpressions(self, context: &mut Context) -> Self {
		match self.value(context).clone() {
			Storable::Expression(expression) => expression.evaluate_subexpressions(context).store(context),
			Storable::Pointer(pointer) => pointer.evaluate_subexpressions(context),
			Storable::Uninitialized => Pointer(0, Span::none()),
			Storable::Object(_) => todo!(),
		}
	}
}

impl Dot for Pointer {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer> {
		match self.value(context).clone() {
			Storable::Expression(expression) => match expression {
				Expression::Text(text) => text.dot(context, name),
				Expression::List(list) => list.dot(context, name),
				Expression::Number(number) => number.dot(context, name),
				Expression::Identifier(identifier) => identifier.dot(context, name),
				Expression::Action(action) => action.dot(context, name),
				Expression::Group(group) => group.dot(context, name),
				Expression::Either(either) => either.dot(context, name),
				Expression::New(new_expression) => new_expression.dot(context, name),
				Expression::Extend(extend) => extend.dot(context, name),

				// non literal
				_ => Some(
					Expression::FieldAccess(FieldAccess {
						left: self.to_owned(),
						right: Identifier::create_virtual(name.to_owned(), context),
						tokens: FieldAccessTokens {
							operator: Token::create_virtual(TokenType::Dot, ".".to_owned()),
						},
					})
					.store(context),
				),
			},
			Storable::Pointer(pointer) => pointer.dot(context, name),
			Storable::Object(object) => object.dot(context, name),

			Storable::Uninitialized => None,
		}
	}
}
