use std::collections::HashMap;

use crate::{
	api::{
		context::Context,
		scope::{ScopeId, ScopeType},
	},
	ast::{
		expressions::{
			field_access::FieldAccessType,
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::{Field, Fields as _, InternalFieldValue},
			parameter::Parameter,
			Expression,
			Spanned,
		},
		misc::tag::TagList,
	},
	comptime::{memory::VirtualPointer, CompileTime},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	if_then_some,
	lexer::{Span, TokenType},
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
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

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordExtend)?.span;
		let outer_scope_id = context.scope_tree.unique_id();

		context.scope_tree.enter_new_scope(ScopeType::RepresentAs);
		let inner_scope_id = context.scope_tree.unique_id();

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens, context)?;
				let name = Parameter::from_literal(parameter.virtual_deref(context)).unwrap().name().to_owned();
				if let Err(error) = context.scope_tree.declare_new_variable(name.clone(), Expression::Pointer(parameter)) {
					context.add_diagnostic(Diagnostic {
						span: name.span(context),
						info: DiagnosticInfo::Error(error),
					});
				};
				parameters.push(parameter);
			});
			parameters
		});

		let type_to_extend = Box::new(Expression::parse(tokens, context));

		let type_to_be = if_then_some!(tokens.next_is(TokenType::KeywordToBe), {
			let _ = tokens.pop(TokenType::KeywordToBe)?;
			Box::new(Expression::parse(tokens, context))
		});

		let mut fields = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			// Parse tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

			// Name
			let name = Name::try_parse(tokens, context)?;

			// Value
			let _ = tokens.pop(TokenType::Equal)?;
			let mut value = Expression::parse(tokens, context);

			// Set tags
			if let Some(tags) = tags {
				value.set_tags(tags, context);
			}

			// Add field
			fields.add_or_overwrite_field(Field {
				name,
				value: Some(value),
				field_type: None,
			});
		})
		.span;

		context.scope_tree.exit_scope().unwrap();

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
		.store_in_memory(context))
	}
}

impl CompileTime for Extend {
	type Output = Extend;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let scope_reverter = context.scope_tree.set_current_scope(self.inner_scope_id);

		let type_to_extend = Box::new(self.type_to_extend.evaluate_at_compile_time(context));
		let type_to_be = self.type_to_be.map(|to_be| Box::new(to_be.evaluate_at_compile_time(context)));

		let mut fields = Vec::new();
		for field in self.fields {
			let field_value = field.value.unwrap().evaluate_at_compile_time(context);

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
			.map(|parameter| parameter.evaluate_at_compile_time(context))
			.collect::<Vec<_>>();

		scope_reverter.revert(context);
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
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}
