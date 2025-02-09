use std::collections::HashMap;

use crate::{
	api::{
		context::context,
		scope::{ScopeId, ScopeType},
		traits::TryAs,
	},
	comptime::{memory::VirtualPointer, CompileTime},
	if_then_else_default,
	if_then_some,
	lexer::{Span, TokenType},
	parse_list,
	parser::{
		expressions::{
			field_access::FieldAccessType,
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::{Field, Fields as _, InternalFieldValue},
			parameter::Parameter,
			Expression,
			Spanned,
			Typed,
		},
		statements::tag::TagList,
		ListType,
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
};

///
/// Normal extension:
///
/// ```cabin
/// let Square = extend Number {
///     square = action(this: Number) {
///         it is this * this;
///     };
/// };
/// ```
///
/// Extension to another type:
///
/// ```cabin
/// let AddPoints = extend Point tobe AddableTo<Point, Point> {
///     plus = action(this: Point, other: Point): Point {
///         it is new Point {
///             x = this.x + other.x,
///             y = this.y + other.y
///         };
///     };
/// };
/// ```
#[derive(Debug, Clone)]
pub struct Extend {
	type_to_extend: Box<Expression>,
	type_to_be: Option<Box<Expression>>,
	fields: Vec<Field>,
	name: Name,
	span: Span,
	compile_time_parameters: Vec<VirtualPointer>,
	inner_scope_id: ScopeId,
	outer_scope_id: ScopeId,
}

impl TryParse for Extend {
	type Output = VirtualPointer;

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Diagnostic> {
		let start = tokens.pop(TokenType::KeywordExtend)?.span;
		let outer_scope_id = context().scope_data.unique_id();

		context().scope_data.enter_new_scope(ScopeType::RepresentAs);
		let inner_scope_id = context().scope_data.unique_id();

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens)?;
				context().scope_data.declare_new_variable(
					Parameter::from_literal(parameter.virtual_deref()).unwrap().name().to_owned(),
					Expression::Pointer(parameter),
				)?;
				parameters.push(parameter);
			});
			parameters
		});

		let type_to_extend = Box::new(Expression::parse(tokens));

		let type_to_be = if_then_some!(tokens.next_is(TokenType::KeywordToBe), {
			let _ = tokens.pop(TokenType::KeywordToBe)?;
			Box::new(Expression::parse(tokens))
		});

		let mut fields = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			// Parse tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens)?);

			// Name
			let name = Name::try_parse(tokens)?;

			// Value
			let _ = tokens.pop(TokenType::Equal)?;
			let mut value = Expression::parse(tokens);

			// Set tags
			if let Some(tags) = tags {
				value.set_tags(tags);
			}

			// Add field
			fields.add_or_overwrite_field(Field {
				name,
				value: Some(value),
				field_type: None,
			});
		})
		.span;

		context().scope_data.exit_scope().unwrap();

		Ok(Extend {
			type_to_extend,
			type_to_be,
			fields,
			span: start.to(end),
			name: "anonymous_represent_as".into(),
			compile_time_parameters,
			inner_scope_id,
			outer_scope_id,
		}
		.to_literal()
		.store_in_memory())
	}
}

impl CompileTime for Extend {
	type Output = Extend;

	fn evaluate_at_compile_time(self) -> Self::Output {
		let _scope_reverter = context().scope_data.set_current_scope(self.inner_scope_id);

		let type_to_extend = Box::new(self.type_to_extend.evaluate_at_compile_time());
		let type_to_be = self.type_to_be.map(|to_be| Box::new(to_be.evaluate_at_compile_time()));

		let mut fields = Vec::new();
		for field in self.fields {
			let field_value = field.value.unwrap().evaluate_at_compile_time();

			fields.add_or_overwrite_field(Field {
				name: field.name,
				value: Some(field_value),
				field_type: None,
			});
		}

		// Evaluate compile-time parameters
		let compile_time_parameters = self
			.compile_time_parameters
			.into_iter()
			.map(|parameter| parameter.evaluate_at_compile_time())
			.collect::<Vec<_>>();

		Extend {
			type_to_extend,
			type_to_be,
			name: self.name,
			span: self.span,
			fields,
			inner_scope_id: self.inner_scope_id,
			outer_scope_id: self.outer_scope_id,
			compile_time_parameters,
		}
	}
}

impl Extend {
	pub const fn type_to_represent(&self) -> &Expression {
		&self.type_to_extend
	}

	pub fn type_to_represent_as(&self) -> Option<&Expression> {
		self.type_to_be.as_ref().map(|type_to_be| type_to_be.as_ref())
	}

	pub fn fields(&self) -> &[Field] {
		&self.fields
	}

	pub fn can_represent(&self, object: &Expression) -> anyhow::Result<bool> {
		let _scope_reverter = context().scope_data.set_current_scope(self.inner_scope_id);

		if let Expression::Pointer(pointer) = self.type_to_extend.as_ref() {
			let literal = pointer.virtual_deref();
			if literal.type_name() == &"Parameter".into() {
				let parameter = Parameter::from_literal(literal).unwrap();
				let anything: VirtualPointer = *context().scope_data.get_variable("Anything").unwrap().try_as::<VirtualPointer>()?;
				let parameter_type = parameter.get_type()?;
				if parameter_type == anything || object.is_assignable_to_type(parameter_type)? {
					return Ok(true);
				}
			}
		}

		Ok(false)
	}

	pub fn representables(&self) -> anyhow::Result<String> {
		let _scope_reverter = context().scope_data.set_current_scope(self.inner_scope_id);

		if let Expression::Name(name) = self.type_to_extend.as_ref() {
			if let Expression::Parameter(parameter) = context().scope_data.get_variable(name).unwrap() {
				let parameter_type = parameter.get_type()?;
				return Ok(parameter_type.virtual_deref().name().unmangled_name().to_owned());
			}
		}

		Ok("unknown".to_owned())
	}

	pub fn set_name(&mut self, name: Name) {
		self.name = name.clone();
	}
}

impl LiteralConvertible for Extend {
	fn to_literal(self) -> LiteralObject {
		LiteralObject {
			address: None,
			fields: HashMap::from([]),
			internal_fields: HashMap::from([
				("fields".to_owned(), InternalFieldValue::FieldList(self.fields)),
				("type_to_represent".to_owned(), InternalFieldValue::Expression(*self.type_to_extend)),
				(
					"type_to_represent_as".to_owned(),
					InternalFieldValue::OptionalExpression(self.type_to_be.map(|inner| *inner)),
				),
				("compile_time_parameters".to_owned(), InternalFieldValue::PointerList(self.compile_time_parameters)),
			]),
			name: self.name,
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: self.outer_scope_id,
			inner_scope_id: Some(self.inner_scope_id),
			span: self.span,
			type_name: "RepresentAs".into(),
			tags: TagList::default(),
		}
	}

	fn from_literal(literal: &LiteralObject) -> anyhow::Result<Self> {
		Ok(Extend {
			fields: literal.get_internal_field::<Vec<Field>>("fields")?.to_owned(),
			type_to_extend: Box::new(literal.get_internal_field::<Expression>("type_to_represent")?.to_owned()),
			type_to_be: literal
				.get_internal_field::<Option<Expression>>("type_to_represent_as")?
				.to_owned()
				.map(|inner| Box::new(inner)),
			compile_time_parameters: literal.get_internal_field::<Vec<VirtualPointer>>("compile_time_parameters")?.to_owned(),
			outer_scope_id: literal.outer_scope_id(),
			inner_scope_id: literal.inner_scope_id.unwrap(),
			name: literal.name.clone(),
			span: literal.span,
		})
	}
}

impl Spanned for Extend {
	fn span(&self) -> Span {
		self.span
	}
}
