use std::{collections::HashMap, fmt::Write as _};

use derive_getters::Getters;
use try_as::{
	macros::{From, TryAsRef},
	traits::{self as try_as_traits, TryAsMut, TryAsRef},
};

use crate::{
	Context,
	Span,
	SpanLike,
	Spanned,
	ast::expressions::Expression,
	comptime::{
		Dot,
		storage::{Pointer, Storable},
		types::{EvaluatedInnerType, EvaluatedType},
	},
	traits::TryAs as _,
};

#[derive(Debug, Clone, TryAsRef, try_as::macros::TryAsMut, From)]
pub enum InternalFieldValue {
	ExpressionList(Vec<Pointer>),
	Number(f64),
	String(String),
}

impl InternalFieldValue {
	pub fn is_fully_known(&self, context: &Context) -> bool {
		match self {
			Self::ExpressionList(values) => values.iter().all(|value| value.is_fully_known(context)),
			_ => true,
		}
	}
}

#[derive(Debug, Clone, Getters)]
pub struct Object {
	fields: HashMap<String, Pointer>,
	internal_fields: HashMap<&'static str, InternalFieldValue>,

	#[getter(skip)]
	pub span: Span,
	pub base_type: Option<EvaluatedType>,
}

impl Object {
	pub fn store(self, context: &mut Context) -> Pointer {
		context.storage.store(self)
	}

	pub fn new(base_type: Option<EvaluatedType>, fields: HashMap<String, Pointer>, span: Span) -> Object {
		Object {
			fields,
			internal_fields: HashMap::new(),
			span,
			base_type,
		}
	}

	pub fn with_defaults(
		group: Pointer,
		mut fields: HashMap<String, Pointer>,
		internal_fields: HashMap<&'static str, InternalFieldValue>,
		context: &Context,
		span: Span,
	) -> Object {
		let binding = group.value(context);
		let Some(Expression::Group(inner_group)) = binding.as_expression() else {
			unreachable!()
		};

		for field in inner_group.fields() {
			if let Some(value) = field.default_value() {
				let name = field.name().source_identifier().to_owned();
				let _ = fields.entry(name).or_insert_with(|| value.to_owned());
			}
		}

		Object {
			base_type: Some(EvaluatedType {
				editable: false,
				inner: EvaluatedInnerType::Group(group),
				span: Span::none(),
			}),
			fields,
			internal_fields,
			span,
		}
	}

	pub fn get_field_value(&self, name: &str) -> Option<Pointer> {
		self.fields.iter().find_map(|(other, value)| (name == other).then_some(value.to_owned()))
	}

	pub fn get_internal_field<T>(&self, name: &str) -> Option<&T>
	where
		InternalFieldValue: TryAsRef<T>,
	{
		self.internal_fields.get(name).and_then(|value| value.try_as::<T>().ok())
	}

	pub fn get_internal_field_mut<T>(&mut self, name: &str) -> Option<&mut T>
	where
		InternalFieldValue: TryAsMut<T>,
	{
		self.internal_fields.get_mut(name).and_then(|value| value.try_mut::<T>().ok())
	}

	pub fn to_text(&self, context: &Context) -> String {
		let mut builder = "{".to_owned();
		for (name, value) in &self.fields {
			write!(builder, "{name} = ").unwrap();
			todo!();
			for line in value.value(context).as_object().unwrap().to_text(context).lines() {
				write!(builder, "\t{line}").unwrap();
			}
			builder += "}";
		}
		builder
	}
}

impl Spanned for Object {
	fn span(&self) -> impl SpanLike {
		&self.span
	}
}

impl Dot for Object {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer> {
		if let Some(value) = self.get_field_value(name) {
			return Some(value);
		}

		if let Some(base_type) = &self.base_type {
			match &base_type.inner {
				EvaluatedInnerType::Group(pointer) => {
					let Storable::Expression(Expression::Group(group)) = pointer.value(context) else {
						unreachable!();
					};

					if let Some(field) = group.fields().iter().find(|field| field.name().source_identifier() == name)
						&& let Some(value) = field.default_value()
					{
						if let Storable::Expression(Expression::Action(action)) = value.value(context).clone() {
							return Some(Expression::from(action.bind(self.clone(), context)).store(context));
						}

						return Some(value.to_owned());
					}
				},
				_ => todo!(),
			}
		}

		None
	}
}
