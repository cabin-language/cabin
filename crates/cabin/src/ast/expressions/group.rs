use std::collections::HashMap;

use convert_case::{Case, Casing as _};
use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{Expression, identifier::Identifier, object::Object},
		misc::{compile_time_parameter::CompileTimeParameters, tag::TagList},
	},
	comptime::{
		Dot,
		Evaluate,
		storage::Pointer,
		types::{EvaluatedType, Type},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct Group {
	compile_time_parameters: Option<CompileTimeParameters>,
	fields: Vec<GroupField>,
	tokens: GroupTokens,
	object: Pointer,
	is_evaluated: bool,
}

#[derive(Debug, Clone, Getters)]
pub struct GroupTokens {
	keyword_group: Token,
	left_brace: Token,
	right_brace: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct GroupField {
	tags: Option<TagList>,
	name: Identifier,
	field_type: Option<Type>,
	default_value: Option<Pointer>,
	tokens: GroupFieldTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct GroupFieldTokens {
	colon: Option<Token>,
	equals: Option<Token>,
	comma: Option<Token>,
	visible: Option<Token>,
}

impl TryParse for Group {
	type Output = Group;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		log!(context, "Parsing group");
		let keyword_group = tokens.pop(TokenType::KeywordGroup, context)?;
		let group_scope = context.scope.enter_new_scope(ScopeType::Group);

		let compile_time_parameters = CompileTimeParameters::try_parse(tokens, context).ok();
		if let Some(compile_time_parameters) = &compile_time_parameters {
			for parameter in compile_time_parameters.parameters() {
				let declaration = parameter.as_declaration(context);
				if let Err(error) = context.scope.declare_new_variable_from_id(parameter.name(), declaration, group_scope) {
					context.add_diagnostic(Diagnostic::at(parameter.name(), error));
				};
			}
		}

		let left_brace = tokens.pop(TokenType::LeftBrace, context)?;

		// Fields
		let mut fields = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
			log!(context, "Parsing group field");
			//  Group field tags
			let tags = if_then_some!(
				tokens.next_is(TokenType::TagOpening),
				TagList::try_parse(tokens, context).inspect_err(|_err| {
					context.scope.exit_scope(group_scope).unwrap();
				})?
			);

			// field modifiers
			let visible = if_then_some!(tokens.next_is(TokenType::KeywordVisible), tokens.pop(TokenType::KeywordVisible, context).unwrap());

			// field name
			let name = Identifier::try_parse(tokens, context).inspect_err(|_err| {
				context.scope.exit_scope(group_scope).unwrap();
			})?;

			if !name.source_identifier().is_case(Case::Snake) {
				context.add_diagnostic_at(&name, DiagnosticInfo::NonSnakeCaseName {
					original_name: name.source_identifier().to_owned(),
				});
			}

			// check for duplicate fields
			if fields.iter().any(|field: &GroupField| field.name == name) {
				context.add_diagnostic_at(&name, DiagnosticInfo::DuplicateGroupField(name.source_identifier().to_owned()));
			}

			// field type
			let (colon, field_type) = if_then_some!(tokens.next_is(TokenType::Colon), {
				(
					tokens.pop(TokenType::Colon, context).inspect_err(|_err| {
						context.scope.exit_scope(group_scope).unwrap();
					})?,
					Type::try_parse(tokens, context)?,
				)
			})
			.unzip();

			// field value
			let (equals, value) = if_then_some!(tokens.next_is(TokenType::Equal), {
				(
					tokens.pop(TokenType::Equal, context).inspect_err(|_err| {
						context.scope.exit_scope(group_scope).unwrap();
					})?,
					Expression::try_parse(tokens, context)?,
				)
			})
			.unzip();

			// comma
			let comma = tokens.pop(TokenType::Comma, context).ok();
			let has_comma = comma.is_some();

			// add field
			fields.push(GroupField {
				tags,
				name,
				default_value: value,
				field_type,
				tokens: GroupFieldTokens { colon, equals, comma, visible },
			});

			if !has_comma {
				break;
			}
		}

		let right_brace = tokens.pop(TokenType::RightBrace, context)?;

		context.scope.exit_scope(group_scope).unwrap();

		Ok(Group {
			object: Object::new(Some(EvaluatedType::builtin("Group", context)), HashMap::new(), keyword_group.span.to(&right_brace.span)).store(context),
			fields,
			tokens: GroupTokens {
				keyword_group,
				left_brace,
				right_brace,
			},
			compile_time_parameters,
			is_evaluated: false,
		})
	}
}

impl Evaluate for Group {
	type Output = Group;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		if self.is_evaluated {
			return self;
		}

		log!(context, "Evaluating group");
		let span = self.span().owned();
		let mut fields = Vec::new();

		for field in self.fields {
			log!(context, "Evaluating group field \"{}\"", field.name.source_identifier());
			let tags = field.tags.evaluate(context);
			log!(context, "Evaluating group field \"{}\" value", field.name.source_identifier());
			let default_value = field.default_value.evaluate(context);
			log!(context, "Evaluating group field \"{}\" type", field.name.source_identifier());
			let field_type = field.field_type.evaluate(context).map(Type::Evaluated);

			fields.push(GroupField {
				name: field.name,
				tokens: field.tokens,
				tags,
				default_value,
				field_type,
			});
		}

		log!(context, "Creating evaluated group");

		Group {
			object: self.object,
			fields,
			compile_time_parameters: self.compile_time_parameters.evaluate(context),
			is_evaluated: true,
			tokens: self.tokens,
		}
	}
}

impl Spanned for Group {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_group().span().to(self.tokens().right_brace().span())
	}
}

impl Group {
	pub fn is_fully_known(&self, context: &Context) -> bool {
		self.fields
			.iter()
			.all(|field| field.default_value.as_ref().is_none_or(|value| value.is_fully_known(context)))
			&& self.compile_time_parameters.as_ref().is_none_or(|parameters| parameters.is_fully_known(context))
	}
}

impl Dot for Group {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer> {
		self.object.dot(context, name)
	}
}
