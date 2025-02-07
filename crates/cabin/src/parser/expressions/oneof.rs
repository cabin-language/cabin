use std::collections::HashMap;

use super::field_access::FieldAccessType;
use crate::{
	api::{
		context::context,
		scope::{ScopeId, ScopeType},
	},
	comptime::{memory::VirtualPointer, CompileTime},
	if_then_else_default,
	lexer::{Span, TokenType},
	parse_list,
	parser::{
		expressions::{
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::InternalFieldValue,
			Expression,
			Spanned,
		},
		statements::tag::TagList,
		ListType,
		Parse,
		TokenQueue,
		TokenQueueFunctionality,
	},
};

#[derive(Debug, Clone)]
pub struct OneOf {
	compile_time_parameters: Vec<Name>,
	choices: Vec<Expression>,
	outer_scope_id: ScopeId,
	inner_scope_id: ScopeId,
	span: Span,
	name: Name,
}

impl Parse for OneOf {
	type Output = VirtualPointer;

	fn parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Error> {
		let start = tokens.pop(TokenType::KeywordOneOf)?.span;

		// Enter inner scope
		context().scope_data.enter_new_scope(ScopeType::OneOf);
		let inner_scope_id = context().scope_data.unique_id();

		// Compile-time parameters
		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			let _ = parse_list!(tokens, ListType::AngleBracketed, {
				let name = Name::parse(tokens)?;
				context().scope_data.declare_new_variable(name.clone(), Expression::Void(()))?;
				compile_time_parameters.push(name);
			});
			compile_time_parameters
		});

		// Choices
		let mut choices = Vec::new();
		let end = parse_list!(tokens, ListType::Braced, {
			choices.push(Expression::parse(tokens)?);
		})
		.span;

		// Exit the scope
		context().scope_data.exit_scope().unwrap();

		// Return
		Ok(OneOf {
			choices,
			compile_time_parameters,
			outer_scope_id: context().scope_data.unique_id(),
			inner_scope_id,
			span: start.to(end),
			name: "anonymous_one_of".into(),
		}
		.to_literal()
		.store_in_memory())
	}
}

impl CompileTime for OneOf {
	type Output = OneOf;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let mut choices = Vec::new();
		for choice in self.choices {
			if let Expression::Name(choice_name) = &choice {
				if self.compile_time_parameters.contains(choice_name) {
					choices.push(choice);
					continue;
				}
			}

			let choice_value = choice.evaluate_at_compile_time()?;
			choices.push(choice_value);
		}

		Ok(OneOf {
			choices,
			outer_scope_id: self.outer_scope_id,
			inner_scope_id: self.inner_scope_id,
			compile_time_parameters: self.compile_time_parameters,
			span: self.span,
			name: self.name,
		})
	}
}

impl LiteralConvertible for OneOf {
	fn to_literal(self) -> LiteralObject {
		LiteralObject {
			address: None,
			fields: HashMap::from([]),
			internal_fields: HashMap::from([
				("choices".to_owned(), InternalFieldValue::ExpressionList(self.choices)),
				("compile_time_parameters".to_owned(), InternalFieldValue::NameList(self.compile_time_parameters)),
			]),
			name: self.name,
			field_access_type: FieldAccessType::Normal,
			outer_scope_id: self.outer_scope_id,
			inner_scope_id: Some(self.inner_scope_id),
			span: self.span,
			type_name: "OneOf".into(),
			tags: TagList::default(),
		}
	}

	fn from_literal(literal: &LiteralObject) -> anyhow::Result<Self> {
		Ok(OneOf {
			choices: literal.get_internal_field::<Vec<Expression>>("choices")?.to_owned(),
			compile_time_parameters: literal.get_internal_field::<Vec<Name>>("compile_time_parameters")?.to_owned(),
			outer_scope_id: literal.outer_scope_id(),
			inner_scope_id: literal.inner_scope_id.unwrap(),
			span: literal.span,
			name: literal.name().to_owned(),
		})
	}
}

impl Spanned for OneOf {
	fn span(&self) -> Span {
		self.span
	}
}
