use std::{
	collections::{HashMap, hash_map},
	fmt::Debug,
	marker::PhantomData,
};

use try_as::macros::From;

use crate::{
	Context,
	Span,
	SpanLike,
	Spanned,
	ast::expressions::{Expression, evaluated::EvaluatedExpression, object::Object},
	comptime::{Dot, Evaluate},
};

#[derive(Debug, Clone)]
pub enum Storable {
	Expression(Expression),
	Evaluated(Evaluated),
	Uninitialized,
}

impl Spanned for Storable {
	fn span(&self) -> impl SpanLike {
		match self {
			Storable::Expression(expression) => expression.span().owned(),
			Storable::Uninitialized => Span::none(),
			Storable::Evaluated(evaluated) => evaluated.span().owned(),
		}
	}
}

impl From<Object> for Storable {
	fn from(expression: Object) -> Self {
		Storable::Evaluated(Evaluated::Object(expression))
	}
}

impl<'ctx> TryFrom<&'ctx Storable> for &'ctx Object {
	type Error = ();

	fn try_from(value: &'ctx Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Evaluated(Evaluated::Object(object)) => Ok(object),
			_ => Err(()),
		}
	}
}

impl<'ctx> TryFrom<&'ctx mut Storable> for &'ctx mut Object {
	type Error = ();

	fn try_from(value: &'ctx mut Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Evaluated(Evaluated::Object(object)) => Ok(object),
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

#[derive(Debug, Clone, From)]
pub enum Evaluated {
	Expression(EvaluatedExpression),
	Object(Object),
	Pointer(Pointer<Evaluated>),
}

impl Spanned for Evaluated {
	fn span(&self) -> impl SpanLike {
		match self {
			Evaluated::Expression(evaluated_expression) => evaluated_expression.span().owned(),
			Evaluated::Object(object) => object.span().owned(),
			Evaluated::Pointer(pointer) => pointer.span().owned(),
		}
	}
}

impl Pointer<Evaluated> {
	pub fn is_fully_known(&self, context: &Context) -> bool {
		match self.value(context) {
			Evaluated::Expression(evaluated_expression) => evaluated_expression.is_fully_known(context),
			Evaluated::Object(object) => object.is_fully_known(context),
			Evaluated::Pointer(pointer) => pointer.is_fully_known(context),
		}
	}

	pub fn as_string<'ctx>(&'ctx self, context: &'ctx Context) -> Option<&'ctx str> {
		self.as_object(context).and_then(|object| object.get_text())
	}

	pub fn as_number<'ctx>(&'ctx self, context: &'ctx Context) -> Option<f64> {
		self.as_object(context).and_then(|object| object.get_number())
	}

	pub fn as_list<'ctx>(&'ctx self, context: &'ctx Context) -> Option<&Vec<Pointer<Evaluated>>> {
		self.as_object(context).and_then(|object| object.list_elements())
	}

	pub fn as_mut_list<'ctx>(&'ctx self, context: &'ctx mut Context) -> Option<&mut Vec<Pointer<Evaluated>>> {
		self.as_object_mut(context).and_then(|object| object.list_elements_mut())
	}

	pub fn as_object_mut<'ctx>(&self, context: &'ctx mut Context) -> Option<&'ctx mut Object> {
		self.value_mut(context).as_object_mut(context)
	}

	pub fn as_object<'ctx>(&'ctx self, context: &'ctx Context) -> Option<&'ctx Object> {
		self.value(context).as_object(context)
	}
}

impl Dot for Evaluated {
	fn dot(&self, context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		match self {
			Evaluated::Expression(evaluated_expression) => evaluated_expression.dot(context, name),
			Evaluated::Object(object) => object.dot(context, name),
			Evaluated::Pointer(pointer) => pointer.value(context).dot(context, name),
		}
	}
}

impl Dot for Pointer<Evaluated> {
	fn dot(&self, context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		self.value(context).dot(context, name)
	}
}

impl Dot for Pointer<Object> {
	fn dot(&self, context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		self.value(context).dot(context, name)
	}
}

impl Dot for Pointer<EvaluatedExpression> {
	fn dot(&self, context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		self.value(context).dot(context, name)
	}
}

impl Evaluated {
	pub fn as_object<'ctx>(&'ctx self, context: &'ctx Context) -> Option<&'ctx Object> {
		match self {
			Evaluated::Object(object) => Some(object),
			Evaluated::Expression(_) => None,
			Evaluated::Pointer(inner) => inner.value(context).as_object(context),
		}
	}

	pub fn as_object_mut<'ctx>(&'ctx mut self, context: &'ctx mut Context) -> Option<&'ctx mut Object> {
		match self {
			Evaluated::Object(object) => Some(object),
			Evaluated::Expression(_) => None,
			Evaluated::Pointer(inner) => inner.value_mut(context).as_object_mut(context),
		}
	}

	pub fn as_expression<'ctx>(&'ctx self, context: &'ctx Context) -> Option<&'ctx EvaluatedExpression> {
		match self {
			Evaluated::Object(_) => None,
			Evaluated::Expression(expression) => Some(expression),
			Evaluated::Pointer(inner) => inner.value(context).as_expression(context),
		}
	}
}

impl<'ctx> TryFrom<&'ctx Storable> for &'ctx EvaluatedExpression {
	type Error = ();

	fn try_from(value: &'ctx Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Evaluated(Evaluated::Expression(expression)) => Ok(expression),
			_ => Err(()),
		}
	}
}

impl<'ctx> TryFrom<&'ctx mut Storable> for &'ctx mut EvaluatedExpression {
	type Error = ();

	fn try_from(value: &'ctx mut Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Evaluated(Evaluated::Expression(expression)) => Ok(expression),
			_ => Err(()),
		}
	}
}

impl From<EvaluatedExpression> for Storable {
	fn from(value: EvaluatedExpression) -> Self {
		Storable::Evaluated(value.into())
	}
}

impl<'ctx> TryFrom<&'ctx mut Storable> for &'ctx mut Evaluated {
	type Error = ();

	fn try_from(value: &'ctx mut Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Evaluated(evaluated) => Ok(evaluated),
			_ => Err(()),
		}
	}
}

impl<'ctx> TryFrom<&'ctx Storable> for &'ctx Evaluated {
	type Error = ();

	fn try_from(value: &'ctx Storable) -> Result<Self, Self::Error> {
		match value {
			Storable::Evaluated(evaluated) => Ok(evaluated),
			_ => Err(()),
		}
	}
}

impl From<Evaluated> for Storable {
	fn from(value: Evaluated) -> Self {
		Storable::Evaluated(value)
	}
}

impl From<Pointer<Object>> for Pointer<Evaluated> {
	fn from(value: Pointer<Object>) -> Self {
		Pointer(value.0, value.1, PhantomData)
	}
}

impl From<Pointer<EvaluatedExpression>> for Pointer<Evaluated> {
	fn from(value: Pointer<EvaluatedExpression>) -> Self {
		Pointer(value.0, value.1, PhantomData)
	}
}

#[derive(Debug, Clone)]
pub struct Pointer<T: Into<Storable>>(usize, Span, PhantomData<T>);

impl<T: Into<Storable>> Pointer<T>
where
	for<'ctx> &'ctx T: TryFrom<&'ctx Storable>,
	for<'ctx> <&'ctx T as TryFrom<&'ctx Storable>>::Error: Debug,
	for<'ctx> &'ctx mut T: TryFrom<&'ctx mut Storable>,
	for<'ctx> <&'ctx mut T as TryFrom<&'ctx mut Storable>>::Error: Debug,
{
	pub fn value<'ctx>(&self, context: &'ctx Context) -> &'ctx T {
		context.storage.get(self)
	}

	pub fn value_mut<'ctx>(&self, context: &'ctx mut Context) -> &'ctx mut T {
		context.storage.get_mut(self)
	}

	pub fn with_span(&self, span: Span) -> Self {
		Pointer(self.0, span, PhantomData)
	}
}

impl<T: Into<Storable> + Clone> Evaluate for Pointer<T>
where
	for<'ctx> &'ctx T: TryFrom<&'ctx Storable>,
	for<'ctx> <&'ctx T as TryFrom<&'ctx Storable>>::Error: Debug,
{
	type Output = Pointer<Evaluated>;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let storable: Storable = context.storage.get(&self).clone().into();
		let evaluated = match storable {
			Storable::Expression(Expression::Unevaluated(expression)) => expression.evaluate(context),
			Storable::Expression(Expression::Evaluated(_)) | Storable::Evaluated(_) | Storable::Uninitialized => Pointer(self.0, self.1, PhantomData),
		};
		context.storage.memory.insert(self.0, Storable::Evaluated(Evaluated::Pointer(evaluated)));
		Pointer(self.0, self.1, PhantomData)
	}
}

impl<T: Into<Storable>> Spanned for Pointer<T> {
	fn span(&self) -> impl SpanLike {
		&self.1
	}
}

impl<T: Into<Storable>> PartialEq for Pointer<T> {
	fn eq(&self, other: &Self) -> bool {
		self.0 == other.0
	}
}

impl<T: Into<Storable>> Eq for Pointer<T> {}

// impl ExpressionRef {
// 	pub const fn with_span(&self, span: Span) -> ExpressionRef {
// 		ExpressionRef(self.0, span)
// 	}

// 	pub fn value<'ctx>(&self, context: &'ctx Context) -> &'ctx Storable {
// 		context.storage.get(self)
// 	}

// 	pub fn value_mut<'ctx>(&self, context: &'ctx mut Context) -> &'ctx mut Storable {
// 		context.storage.memory.get_mut(&self.0).unwrap()
// 	}

// 	pub fn kind_name(&self, context: &Context) -> &'static str {
// 		match self.value(context) {
// 			Storable::Object(_) => "object",
// 			Storable::Expression(expression) => expression.kind_name(),
// 			Storable::Uninitialized => "uninitialized memory",
// 		}
// 	}

// 	pub fn as_string(&self, context: &Context) -> Option<String> {
// 		self.value(context).as_object().unwrap().get_text().map(|s| s.to_owned())
// 	}

// 	pub fn as_number(&self, context: &Context) -> Option<f64> {
// 		self.value(context).as_object().unwrap().get_number()
// 	}

// 	pub fn as_list<'ctx>(&self, context: &'ctx Context) -> Option<&'ctx Vec<ExpressionRef>> {
// 		self.value(context)
// 			.as_expression()
// 			.unwrap()
// 			.try_as::<EvaluatedList>()
// 			.unwrap()
// 			.object()
// 			.value(context)
// 			.as_object()
// 			.unwrap()
// 			.list_elements()
// 	}

// 	pub fn as_mut_list<'ctx>(&self, context: &'ctx mut Context) -> Option<&'ctx mut Vec<ExpressionRef>> {
// 		self.value(context)
// 			.as_expression()
// 			.unwrap()
// 			.try_as::<EvaluatedList>()
// 			.unwrap()
// 			.object()
// 			.to_owned()
// 			.value_mut(context)
// 			.as_object_mut()
// 			.unwrap()
// 			.list_elements_mut()
// 	}

// 	pub fn is_fully_known(&self, context: &Context) -> bool {
// 		match self.value(context) {
// 			Storable::Object(object) => object.is_fully_known(context),
// 			Storable::Expression(expression) => expression.is_fully_known(),
// 			Storable::Uninitialized => false,
// 		}
// 	}

// 	/// Creates a new empty space in memory and returns a pointer to it.
// 	pub fn calloc(context: &mut Context) -> ExpressionRef {
// 		let id = context.storage.next_unused_virtual_address();
// 		let _ = context.storage.memory.insert(id, Storable::Uninitialized);
// 		ExpressionRef(id, Span::none())
// 	}
// }

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

	pub fn store<T: Into<Storable> + Spanned>(&mut self, value: T) -> Pointer<T> {
		let address = self.next_unused_virtual_address();
		let pointer = Pointer(address, value.span().owned(), PhantomData);
		let _ = self.memory.insert(address, value.into());
		pointer
	}

	pub fn calloc(&mut self) -> Pointer<Expression> {
		let id = self.next_unused_virtual_address();
		let _ = self.memory.insert(id, Storable::Uninitialized);
		Pointer(id, Span::none(), PhantomData)
	}

	pub fn get<T: Into<Storable>>(&self, address: &Pointer<T>) -> &T
	where
		for<'ctx> &'ctx T: TryFrom<&'ctx Storable>,
		for<'ctx> <&'ctx T as TryFrom<&'ctx Storable>>::Error: Debug,
	{
		self.memory.get(&address.0).unwrap().try_into().unwrap()
	}

	pub fn get_mut<T: Into<Storable>>(&mut self, address: &Pointer<T>) -> &mut T
	where
		for<'ctx> &'ctx mut T: TryFrom<&'ctx mut Storable>,
		for<'ctx> <&'ctx mut T as TryFrom<&'ctx mut Storable>>::Error: Debug,
	{
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
