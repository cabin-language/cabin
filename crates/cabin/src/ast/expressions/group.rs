use std::collections::HashMap;

use convert_case::{Case, Casing as _};
use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{Expression, identifier::Identifier, object::Object},
		misc::{
			compile_time_parameter::{CompileTimeParameters, EvaluatedCompileTimeParameters},
			tag::{EvaluatedTagList, TagList},
		},
	},
	comptime::{
		Evaluate,
		storage::{Evaluated, Pointer},
	},
	definition::UnevaluatedVariableDeclaration,
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
}

#[derive(Debug, Clone, Getters)]
pub struct GroupTokens {
	keyword_group: Token,
	left_brace: Token,
	right_brace: Token,
}

#[derive(Debug, Clone)]
pub struct GroupField {
	tags: Option<TagList>,
	name: Identifier,
	field_type: Option<Pointer<Expression>>,
	default_value: Option<Pointer<Expression>>,
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
		let keyword_group = tokens.pop(TokenType::KeywordGroup, context)?;
		let group_scope = context.scope.enter_new_scope(ScopeType::Group);

		let compile_time_parameters = CompileTimeParameters::try_parse(tokens, context).ok();
		if let Some(compile_time_parameters) = &compile_time_parameters {
			for parameter in compile_time_parameters.parameters() {
				context.declare_new_variable(UnevaluatedVariableDeclaration::with_type(parameter.name().to_owned(), parameter.parameter_type().clone()));
			}
		}

		let left_brace = tokens.pop(TokenType::LeftBrace, context)?;

		// Fields
		let mut fields = Vec::new();
		while !tokens.next_is(TokenType::RightBrace) {
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
					Expression::try_parse(tokens, context)?,
				)
			})
			.unzip();

			// field value
			let (equals, mut value) = if_then_some!(tokens.next_is(TokenType::Equal), {
				(
					tokens.pop(TokenType::Equal, context).inspect_err(|_err| {
						context.scope.exit_scope(group_scope).unwrap();
					})?,
					Expression::try_parse(tokens, context)?,
				)
			})
			.unzip();

			if let Some(tags) = &tags
				&& let Some(value) = &mut value
			{
				value.as_unevaluated_mut(context).unwrap().set_tags(tags.clone());
			}

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
			fields,
			compile_time_parameters,
			tokens: GroupTokens {
				keyword_group,
				left_brace,
				right_brace,
			},
		})
	}
}

impl Evaluate for Group {
	type Output = EvaluatedGroup;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating group");
		let mut fields = Vec::new();
		let span = self.span().into_owned();

		for field in self.fields {
			log!(context, "Evaluating group field \"{}\"", field.name.source_identifier());
			let tags = field.tags.evaluate(context);
			log!(context, "Evaluating group field \"{}\" value", field.name.source_identifier());
			let default_value = field.default_value.evaluate(context);
			log!(context, "Evaluating group field \"{}\" type", field.name.source_identifier());
			let field_type = field.field_type.evaluate(context);

			fields.push(EvaluatedGroupField {
				name: field.name,
				tokens: field.tokens,
				tags,
				default_value,
				field_type,
			});
		}

		log!(context, "Creating group object");
		let obj = Object::new(
			context.builtin("Group"),
			HashMap::from([(
				"fields".to_owned(),
				Object::list(
					fields
						.iter()
						.map(|field| {
							Object::new(
								context.builtin("Property"),
								HashMap::from([("name".to_owned(), field.name.stringify(context).store(context))]),
								field.name.span().into_owned(),
							)
							.store(context)
						})
						.collect(),
					context,
					span.clone(),
				)
				.store(context),
			)]),
			span,
		)
		.store(context);

		EvaluatedGroup {
			object: obj,
			fields,
			compile_time_parameters: self.compile_time_parameters.evaluate(context),
			tokens: self.tokens,
		}
	}
}

impl Spanned for Group {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_group().span().to(self.tokens().right_brace().span())
	}
}

impl Spanned for EvaluatedGroup {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_group().span().to(self.tokens().right_brace().span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedGroup {
	compile_time_parameters: Option<EvaluatedCompileTimeParameters>,
	fields: Vec<EvaluatedGroupField>,
	tokens: GroupTokens,

	object: Pointer<Object>,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedGroupField {
	tags: Option<EvaluatedTagList>,
	name: Identifier,
	field_type: Option<Pointer<Evaluated>>,
	default_value: Option<Pointer<Evaluated>>,
	tokens: GroupFieldTokens,
}
