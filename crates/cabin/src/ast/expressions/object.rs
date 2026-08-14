use std::{borrow::Cow, collections::HashMap, fmt::Write as _};

use derive_getters::Getters;
use try_as::{macros::TryAsRef, traits as try_as_traits, traits::TryAsMut};

use crate::{
	Context,
	Span,
	SpanLike,
	Spanned,
	ast::expressions::evaluated::EvaluatedExpression,
	comptime::{
		Dot,
		storage::{Evaluated, Pointer},
	},
	traits::TryAs as _,
};

#[derive(Debug, Clone, TryAsRef, try_as::macros::TryAsMut)]
pub enum InternalFieldValue {
	ExpressionList(Vec<Pointer<Evaluated>>),
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
	group: Pointer<Evaluated>,
	fields: HashMap<String, Pointer<Evaluated>>,
	internal_fields: HashMap<&'static str, InternalFieldValue>,

	#[getter(skip)]
	span: Span,
}

impl Object {
	pub fn store(self, context: &mut Context) -> Pointer<Object> {
		context.storage.store(self)
	}

	pub fn text<'str, S: Into<Cow<'str, str>>>(text: S, context: &mut Context, span: Span) -> Object {
		Object {
			group: context.builtin("Text"),
			fields: HashMap::new(),
			internal_fields: HashMap::from([("text", InternalFieldValue::String(text.into().into_owned()))]),
			span,
		}
	}

	pub fn number(value: f64, context: &mut Context, span: Span) -> Object {
		Object {
			group: context.builtin("Number"),
			fields: HashMap::new(),
			internal_fields: HashMap::from([("value", InternalFieldValue::Number(value))]),
			span,
		}
	}

	pub fn list(elements: Vec<Pointer<Evaluated>>, context: &mut Context, span: Span) -> Object {
		Object::with_defaults(
			context.builtin("List"),
			HashMap::new(),
			HashMap::from([("elements", InternalFieldValue::ExpressionList(elements))]),
			context,
			span,
		)
	}

	pub fn get_text(&self) -> Option<&str> {
		self.internal_fields.get("text").map(|value| value.try_as::<String>().unwrap().as_str())
	}

	pub fn get_number(&self) -> Option<f64> {
		self.internal_fields.get("value").map(|value| *value.try_as::<f64>().unwrap())
	}

	pub fn list_elements(&self) -> Option<&Vec<Pointer<Evaluated>>> {
		self.internal_fields.get("elements").map(|value| value.try_as::<Vec<Pointer<Evaluated>>>().unwrap())
	}

	pub fn list_elements_mut(&mut self) -> Option<&mut Vec<Pointer<Evaluated>>> {
		self.internal_fields.get_mut("elements").map(|value| value.try_mut::<Vec<Pointer<Evaluated>>>().unwrap())
	}

	pub fn new(group: Pointer<Evaluated>, fields: HashMap<String, Pointer<Evaluated>>, span: Span) -> Object {
		Object {
			group,
			fields,
			internal_fields: HashMap::new(),
			span,
		}
	}

	pub fn with_defaults(
		group: Pointer<Evaluated>,
		mut fields: HashMap<String, Pointer<Evaluated>>,
		internal_fields: HashMap<&'static str, InternalFieldValue>,
		context: &Context,
		span: Span,
	) -> Object {
		dbg!(group.value(context));
		let binding = group.value(context);
		let Some(EvaluatedExpression::Group(inner_group)) = binding.as_expression(context) else {
			unreachable!()
		};

		for field in inner_group.fields() {
			if let Some(value) = field.default_value() {
				let name = field.name().source_identifier().to_owned();
				let _ = fields.entry(name).or_insert_with(|| value.to_owned());
			}
		}

		Object {
			group,
			fields,
			internal_fields,
			span,
		}
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		self.fields.iter().all(|(_, value)| value.is_fully_known(context)) && self.internal_fields.iter().all(|(_, value)| value.is_fully_known(context))
	}

	pub fn get_field_value(&self, name: &str) -> Option<Pointer<Evaluated>> {
		self.fields.iter().find_map(|(other, value)| (name == other).then_some(value.to_owned()))
	}

	pub fn to_text(&self, context: &Context) -> String {
		if let Some(text) = self.get_text() {
			return text.to_owned();
		}

		if let Some(number) = self.get_number() {
			return format!("{number}");
		}

		if let Some(elements) = self.list_elements() {
			return format!(
				"[{}]",
				elements
					.iter()
					.map(|element| element.value(context).as_object(context).unwrap().to_text(context))
					.collect::<Vec<_>>()
					.join(", ")
			);
		}

		let mut builder = "{".to_owned();
		for (name, value) in &self.fields {
			write!(builder, "{name} = ").unwrap();
			for line in value.value(context).as_object(context).unwrap().to_text(context).lines() {
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
	fn dot(&self, _context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		self.get_field_value(name)
	}
}
