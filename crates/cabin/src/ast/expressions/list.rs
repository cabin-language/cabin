use std::collections::HashMap;

use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, object::Object},
	comptime::{Dot, Evaluate, storage::Pointer, types::EvaluatedType},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct List {
	elements: Vec<ListElement>,
	tokens: ListTokens,
	object: Pointer,
}

#[derive(Debug, Clone, Getters)]
pub struct ListTokens {
	left_bracket: Token,
	right_bracket: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct ListElement {
	value: Pointer,
	tokens: ListElementTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ListElementTokens {
	comma: Option<Token>,
}

impl TryParse for List {
	type Output = List;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let left_bracket = tokens.pop(TokenType::LeftBracket, context)?;
		let mut elements = Vec::new();
		while !tokens.next_is(TokenType::RightBracket) {
			let value = Expression::try_parse(tokens, context)?;
			let comma = tokens.pop(TokenType::Comma, context).ok();
			let has_comma = comma.is_some();
			elements.push(ListElement {
				value,
				tokens: ListElementTokens { comma },
			});
			if !has_comma {
				break;
			}
		}

		let right_bracket = tokens.pop(TokenType::RightBracket, context)?;
		let object = Object::new(Some(EvaluatedType::builtin("List", context)), HashMap::new(), left_bracket.span().to(right_bracket.span())).store(context);

		Ok(List {
			elements,
			object,
			tokens: ListTokens { left_bracket, right_bracket },
		})
	}
}

impl Evaluate for List {
	type Output = List;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let elements = self
			.elements
			.into_iter()
			.map(|element| ListElement {
				value: element.value.evaluate(context),
				tokens: element.tokens,
			})
			.collect::<Vec<_>>();

		List {
			tokens: self.tokens,
			object: self.object,
			elements,
		}
	}
}

impl Spanned for List {
	fn span(&self) -> impl SpanLike {
		self.tokens().left_bracket().span().to(self.tokens().right_bracket().span())
	}
}

impl ToCabin for List {
	fn to_cabin(&self) -> String {
		format!("[{}]", self.elements.to_cabin())
	}
}

impl ToCabin for ListElement {
	fn to_cabin(&self) -> String {
		self.value.to_cabin()
	}
}

impl List {
	pub fn insert(&mut self, index: usize, element: Pointer, context: &mut Context) {
		self.object
			.as_object_mut(context)
			.unwrap()
			.get_internal_field_mut::<Vec<Pointer>>(internal_fields::ELEMENTS)
			.unwrap()
			.insert(index, element);
	}
}

impl Dot for List {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer> {
		self.object.dot(context, name)
	}
}

pub mod internal_fields {
	pub const ELEMENTS: &str = "elements";
}
