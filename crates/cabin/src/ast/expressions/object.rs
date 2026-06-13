use std::{borrow::Cow, collections::HashMap, fmt::Write as _};

use derive_getters::Getters;
use try_as::{macros::TryAsRef, traits as try_as_traits};

use crate::{
	Context,
	Span,
	Spanned,
	comptime::memory::{Pointer, Storable},
	traits::TryAs as _,
};

#[derive(Debug, Clone, TryAsRef)]
pub enum InternalFieldValue {
	ExpressionList(Vec<Pointer>),
	Number(f64),
	String(String),
}

#[derive(Debug, Clone, Getters)]
pub struct Object {
	group: Pointer,
	fields: HashMap<String, Pointer>,
	internal_fields: HashMap<&'static str, InternalFieldValue>,

	#[getter(skip)]
	span: Span,
}

impl Object {
	pub fn store(self, context: &mut Context) -> Pointer {
		context.memory.store(Storable::Object(self))
	}

	pub fn text<'str, S: Into<Cow<'str, str>>>(text: S, context: &Context, span: Span) -> Object {
		Object {
			group: context.ident("Text"),
			fields: HashMap::new(),
			internal_fields: HashMap::from([("text", InternalFieldValue::String(text.into().into_owned()))]),
			span,
		}
	}

	pub fn number(value: f64, context: &Context, span: Span) -> Object {
		Object {
			group: context.ident("Number"),
			fields: HashMap::new(),
			internal_fields: HashMap::from([("value", InternalFieldValue::Number(value))]),
			span,
		}
	}

	pub fn get_text(&self) -> Option<&str> {
		self.internal_fields.get("text").map(|value| value.try_as::<String>().unwrap().as_str())
	}

	pub fn get_number(&self) -> Option<f64> {
		self.internal_fields.get("value").map(|value| *value.try_as::<f64>().unwrap())
	}

	pub fn list(elements: Vec<Pointer>, context: &Context, span: Span) -> Object {
		Object {
			group: context.ident("List"),
			fields: HashMap::new(),
			internal_fields: HashMap::from([("elements", InternalFieldValue::ExpressionList(elements))]),
			span,
		}
	}

	pub fn list_elements(&self) -> Option<&Vec<Pointer>> {
		self.internal_fields.get("elements").map(|value| value.try_as::<Vec<Pointer>>().unwrap())
	}

	pub fn new(group: Pointer, fields: HashMap<String, Pointer>, span: Span) -> Object {
		Object {
			group,
			fields,
			internal_fields: HashMap::new(),
			span,
		}
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		self.fields.iter().all(|(_, value)| value.is_fully_known(context))
	}

	pub fn get_field_value(&self, name: &str) -> Option<Pointer> {
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
					.map(|element| element.value(context).as_object().unwrap().to_text(context))
					.collect::<Vec<_>>()
					.join(", ")
			);
		}

		let mut builder = "{".to_owned();
		for (name, value) in &self.fields {
			write!(builder, "{name} = ").unwrap();
			for line in value.value(context).as_object().unwrap().to_text(context).lines() {
				write!(builder, "\t{line}").unwrap();
			}
			builder += "}";
		}
		builder
	}
}

impl Spanned for Object {
	fn span(&self) -> Span {
		self.span
	}
}
