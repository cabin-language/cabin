use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
	api::{context::context, scope::ScopeId},
	comptime::{memory::VirtualPointer, CompileTime},
	if_then_else_default,
	if_then_some,
	lexer::{Span, TokenType},
	parse_list,
	parser::{
		expressions::{
			literal::LiteralConvertible,
			name::Name,
			object::{Field, Fields as _},
			parameter::Parameter,
			Expression,
			Spanned,
		},
		statements::tag::TagList,
		ListType,
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality,
		TryParse,
	},
};

#[derive(Debug, Clone)]
pub struct DefaultExtend {
	compile_time_parameters: Vec<VirtualPointer>,
	pub type_to_extend: Expression,
	pub type_to_be: Option<Expression>,
	pub id: usize,
	pub fields: Vec<Field>,
}

static DEFAULT_EXTEND_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone)]
pub struct DefaultExtendPointer {
	scope_id: ScopeId,
	id: usize,
	span: Span,
}

impl TryParse for DefaultExtend {
	type Output = DefaultExtendPointer;

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Diagnostic> {
		let start = tokens.pop(TokenType::KeywordDefault)?.span;
		let _ = tokens.pop(TokenType::KeywordExtend)?;

		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens)?;
				context().scope_data.declare_new_variable(
					Parameter::from_literal(parameter.virtual_deref()).unwrap().name().to_owned(),
					Expression::Pointer(parameter),
				)?;
				compile_time_parameters.push(parameter);
			});
			compile_time_parameters
		});

		let type_to_extend = Expression::parse(tokens);

		let type_to_be = if_then_some!(tokens.next_is(TokenType::KeywordToBe), {
			let _ = tokens.pop(TokenType::KeywordToBe)?;
			let type_to_be = Expression::parse(tokens);
			type_to_be
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

		let id = DEFAULT_EXTEND_ID.fetch_add(1, Ordering::Relaxed);

		let extension = DefaultExtend {
			compile_time_parameters,
			type_to_be,
			type_to_extend,
			id,
			fields,
		};

		let _ = tokens.pop(TokenType::Semicolon)?;

		context().scope_data.add_default_extension(extension);

		Ok(DefaultExtendPointer {
			scope_id: context().scope_data.unique_id(),
			id,
			span: start.to(end),
		})
	}
}

impl CompileTime for DefaultExtendPointer {
	type Output = DefaultExtendPointer;

	fn evaluate_at_compile_time(self) -> Self::Output {
		context()
			.scope_data
			.map_default_extension_from_id(self.scope_id, self.id, DefaultExtend::evaluate_at_compile_time);

		self
	}
}

impl CompileTime for DefaultExtend {
	type Output = DefaultExtend;

	fn evaluate_at_compile_time(self) -> Self::Output {
		let type_to_extend = self.type_to_extend.evaluate_at_compile_time();
		let type_to_be = self.type_to_be.map(|to_be| to_be.evaluate_at_compile_time());
		let compile_time_parameters = self
			.compile_time_parameters
			.into_iter()
			.map(|parameter| parameter.evaluate_at_compile_time())
			.collect::<Vec<_>>();

		DefaultExtend {
			type_to_be,
			type_to_extend,
			compile_time_parameters,
			id: self.id,
			fields: self.fields,
		}
	}
}

impl Spanned for DefaultExtendPointer {
	fn span(&self) -> Span {
		self.span
	}
}
