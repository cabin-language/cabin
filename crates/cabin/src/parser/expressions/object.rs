use std::{collections::HashMap, fmt::Write as _};

use try_as::traits as try_as_traits;

use crate::{
	api::{context::context, scope::ScopeId},
	comptime::{memory::VirtualPointer, CompileTime},
	debug_log,
	debug_start,
	if_then_else_default,
	if_then_some,
	lexer::{Span, TokenType},
	mapped_err,
	parse_list,
	parser::{
		expressions::{field_access::FieldAccessType, group::GroupDeclaration, literal::LiteralConvertible as _, name::Name, parameter::Parameter, Expression, Spanned},
		statements::tag::TagList,
		ListType,
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality,
		TryParse,
	},
	transpiler::TranspileToC,
};

#[derive(Debug, Clone)]
pub struct ObjectConstructor {
	pub type_name: Name,
	pub fields: Vec<Field>,
	pub internal_fields: HashMap<String, InternalFieldValue>,
	pub outer_scope_id: ScopeId,
	pub inner_scope_id: ScopeId,
	pub field_access_type: FieldAccessType,
	pub name: Name,
	pub span: Span,
	pub tags: TagList,
}

#[derive(Debug, Clone)]
pub struct Field {
	pub name: Name,
	pub field_type: Option<Expression>,
	pub value: Option<Expression>,
}

pub trait Fields {
	fn add_or_overwrite_field(&mut self, field: Field);
}

impl Fields for Vec<Field> {
	fn add_or_overwrite_field(&mut self, field: Field) {
		while let Some(index) = self.iter().enumerate().find_map(|(index, other)| (other.name == field.name).then_some(index)) {
			let _ = self.remove(index);
		}
		self.push(field);
	}
}

impl ObjectConstructor {
	pub fn untyped(fields: Vec<Field>, span: Span) -> ObjectConstructor {
		ObjectConstructor {
			type_name: "Object".into(),
			name: "anonymous_object".into(),
			field_access_type: FieldAccessType::Normal,
			internal_fields: HashMap::new(),
			inner_scope_id: context().scope_data.unique_id(),
			outer_scope_id: context().scope_data.unique_id(),
			fields,
			tags: TagList::default(),
			span,
		}
	}

	pub fn typed<T: Into<Name>>(type_name: T, fields: Vec<Field>, span: Span) -> ObjectConstructor {
		ObjectConstructor {
			type_name: type_name.into(),
			name: "anonymous_object".into(),
			field_access_type: FieldAccessType::Normal,
			internal_fields: HashMap::new(),
			inner_scope_id: context().scope_data.unique_id(),
			outer_scope_id: context().scope_data.unique_id(),
			fields,
			tags: TagList::default(),
			span,
		}
	}

	pub fn string(string: &str, span: Span) -> ObjectConstructor {
		ObjectConstructor {
			type_name: Name::from("Text"),
			fields: Vec::new(),
			internal_fields: HashMap::from([("internal_value".to_owned(), InternalFieldValue::String(string.to_owned()))]),
			outer_scope_id: context().scope_data.unique_id(),
			inner_scope_id: context().scope_data.unique_id(),
			field_access_type: FieldAccessType::Normal,
			name: Name::non_mangled("anonymous_string_literal"),
			span,
			tags: TagList::default(),
		}
	}

	pub fn number(number: f64, span: Span) -> ObjectConstructor {
		ObjectConstructor {
			type_name: Name::from("Number"),
			fields: Vec::new(),
			internal_fields: HashMap::from([("internal_value".to_owned(), InternalFieldValue::Number(number))]),
			outer_scope_id: context().scope_data.unique_id(),
			inner_scope_id: context().scope_data.unique_id(),
			field_access_type: FieldAccessType::Normal,
			name: "anonymous_number".into(),
			span,
			tags: TagList::default(),
		}
	}

	pub fn is_literal(&self) -> bool {
		for field in &self.fields {
			let value = field.value.as_ref().unwrap();
			if let Expression::Pointer(_) = value {
				continue;
			}

			let Expression::ObjectConstructor(constructor) = value else {
				return false;
			};

			if !constructor.is_literal() {
				return false;
			}
		}

		true
	}

	pub fn get_field<T: Into<Name>>(&self, name: T) -> Option<&Expression> {
		let name = name.into();
		self.fields.iter().find_map(|field| (field.name == name).then(|| field.value.as_ref().unwrap()))
	}
}

impl TryParse for ObjectConstructor {
	type Output = ObjectConstructor;

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Diagnostic> {
		let start = tokens.pop(TokenType::KeywordNew)?.span;

		// Name
		let name = Name::try_parse(tokens)?;

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens)?;
				context()
					.scope_data
					.declare_new_variable(parameter.virtual_deref().name(), Expression::Pointer(parameter))?;
				compile_time_parameters.push(parameter);
			});
			compile_time_parameters
		});

		// Fields
		let mut fields = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			// Parse tags
			let tags = if tokens.next_is(TokenType::TagOpening) {
				Some(TagList::try_parse(tokens)?)
			} else {
				None
			};

			// Name
			let field_name = Name::try_parse(tokens)?;

			// Value
			let _ = tokens.pop(TokenType::Equal)?;
			let mut value = Expression::parse(tokens);

			// Set tags
			if let Some(tags) = tags {
				value.set_tags(tags);
			}

			// Add field
			fields.add_or_overwrite_field(Field {
				name: field_name,
				value: Some(value),
				field_type: None,
			});
		})
		.span;

		// Return
		Ok(ObjectConstructor {
			type_name: name,
			fields,
			outer_scope_id: context().scope_data.unique_id(),
			inner_scope_id: context().scope_data.unique_id(),
			internal_fields: HashMap::new(),
			field_access_type: FieldAccessType::Normal,
			name: Name::non_mangled("anonymous_object"),
			span: start.to(end),
			tags: TagList::default(),
		})
	}
}

impl CompileTime for ObjectConstructor {
	type Output = Expression;

	fn evaluate_at_compile_time(mut self) -> anyhow::Result<Self::Output> {
		let debug_section = debug_start!(
			"{} an object of type {}",
			"Compile-Time Evaluating".green().bold(),
			self.type_name.unmangled_name().yellow()
		);
		let _scope_reverter = context().scope_data.set_current_scope(self.inner_scope_id);

		// Get object type
		let object_type = if_then_some!(!matches!(self.type_name.unmangled_name(), "Group" | "Module" | "Object"), {
			GroupDeclaration::from_literal(
				self.type_name
					.clone()
					.evaluate_at_compile_time()
					.map_err(mapped_err! {
						while = format!("evaluating the type of an object constructor at compile time"),
					})?
					.try_as_literal()
					.map_err(mapped_err! {
						while = format!("interpreting an object constructor's type (\"{}\") as a literal", self.type_name.unmangled_name().bold().cyan()),
					})?,
			)
			.map_err(mapped_err! {
				while = "converting an object constructor's type from a literal to a group declaration",
			})?
		});

		// Default fields
		if let Some(object_type) = object_type {
			for field in object_type.fields() {
				if let Some(value) = &field.value {
					self.fields.add_or_overwrite_field(Field {
						name: field.name.clone(),
						value: Some(value.clone()),
						field_type: None,
					});
				}
			}
		}

		// Explicit fields
		for field in self.fields.clone() {
			debug_log!("{} the object field {}", "Compile-Time Evaluating".green().bold(), field.name.unmangled_name().red());
			let field_value = field.value.clone().unwrap();

			let field_value = field_value.evaluate_at_compile_time().map_err(mapped_err! {
				while = format!(
					"evaluating the value of the field \"{}\" of an object at compile-time",
					field.name.unmangled_name().bold().cyan()
				),
			})?;

			let evaluated_field = Field {
				name: field.name.clone(),
				value: Some(field_value.clone()),
				field_type: None,
			};

			self.fields.add_or_overwrite_field(evaluated_field);
		}

		self.tags = self.tags.evaluate_at_compile_time()?;

		let result = if self.is_literal() {
			let literal = self.try_into()?;
			let address = context().virtual_memory.store(literal);
			Ok(Expression::Pointer(address))
		} else {
			Ok(Expression::ObjectConstructor(self))
		};

		debug_section.finish();
		result
	}
}

#[derive(Debug, Clone, try_as::macros::TryInto, try_as::macros::TryAsRef)]
pub enum InternalFieldValue {
	Number(f64),
	String(String),
	Boolean(bool),
	ExpressionList(Vec<Expression>),
	Expression(Expression),
	OptionalExpression(Option<Expression>),
	FieldList(Vec<Field>),
	NameList(Vec<Name>),
	LiteralMap(Vec<(Name, VirtualPointer)>),
	ParameterList(Vec<Parameter>),
	PointerList(Vec<VirtualPointer>),
	Name(Name),
}

impl TranspileToC for ObjectConstructor {
	fn to_c(&self) -> anyhow::Result<String> {
		// Type name
		let name = if self.type_name == "Object".into() {
			format!("type_{}_UNKNOWN", self.name.to_c()?) // TODO
		} else {
			self.type_name.clone().evaluate_at_compile_time()?.to_c()?
		};

		let mut builder = format!("({name}) {{");

		// Fields
		for field in &self.fields {
			write!(builder, "\n\t.{} = {},", field.name.to_c()?, field.value.as_ref().unwrap().to_c()?).unwrap();
		}

		builder += "\n}";
		Ok(builder)
	}
}

impl Spanned for ObjectConstructor {
	fn span(&self) -> Span {
		self.span
	}
}

#[macro_export]
macro_rules! object {
	(
		$context: expr,
		$type_name: ident {
			$(
				$field_name: ident = $field_value: expr
			),* $(,)?
		}
	) => {
		$crate::parser::expressions::object::ObjectConstructor {
			type_name: stringify!($type_name).into(),
			fields: vec![$($crate::parser::expressions::object::Field {
				name: stringify!($field_name),
				field_type: None,
				value: Some($field_value),
			}),*],
			internal_fields: std::collections::HashMap::new(),
			name: $crate::parser::expressions::name::Name::non_mangled("anonymous_object"),
			span: $crate::lexer::Span::unknown(),
			outer_scope_id: $context.scope_data.unique_id(),
			inner_scope_id: $context.scope_data.unique_id(),
			tags: $crate::parser::statements::tag::TagList::default(),
			field_access_type: $crate::parser::expressions::field_access::FieldAccessType::Normal,
		}
	};
}
