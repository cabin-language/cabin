use derive_getters::Getters;

use crate::{
	api::context::Context,
	ast::expressions::Expression,
	comptime::{Evaluate, storage::Pointer},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct TagList {
	values: Vec<TagListItem>,
	tokens: TagListTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct TagListItem {
	value: Pointer,
	tokens: TagListItemTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct TagListItemTokens {
	comma: Option<Token>,
}

#[derive(Debug, Clone, Getters)]
pub struct TagListTokens {
	tag_list_opening: Token,
	right_bracket: Token,
}

impl TryParse for TagList {
	type Output = TagList;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let tag_list_opening = tokens.pop(TokenType::TagOpening, context)?;

		let mut tags = Vec::new();
		while !tokens.next_is(TokenType::RightBracket) {
			let value = Expression::try_parse(tokens, context)?;
			let comma = tokens.pop(TokenType::Comma, context).ok();
			let has_comma = comma.is_some();

			tags.push(TagListItem {
				value,
				tokens: TagListItemTokens { comma },
			});

			if !has_comma {
				break;
			}
		}

		let right_bracket = tokens.pop(TokenType::RightBracket, context)?;

		Ok(TagList {
			values: tags,
			tokens: TagListTokens { tag_list_opening, right_bracket },
		})
	}
}

impl Evaluate for TagList {
	type Output = TagList;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let mut values = Vec::new();
		for value in self.values {
			values.push(TagListItem {
				value: value.value.evaluate(context),
				tokens: value.tokens,
			});
		}
		TagList { values, tokens: self.tokens }
	}
}

impl ToCabin for TagList {
	fn to_cabin(&self) -> String {
		format!("#[{}]", self.values.to_cabin())
	}
}

impl ToCabin for TagListItem {
	fn to_cabin(&self) -> String {
		self.value.to_cabin()
	}
}
