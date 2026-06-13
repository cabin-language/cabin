use derive_getters::Getters;

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::Expression,
	comptime::{Evaluate, memory::Pointer},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct List {
	elements: Vec<ListElement>,
	tokens: ListTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ListTokens {
	left_bracket: Token,
	right_bracket: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct ListElement {
	value: Expression,
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

		Ok(List {
			elements,
			tokens: ListTokens { left_bracket, right_bracket },
		})
	}
}

impl Evaluate for List {
	type Output = EvaluatedList;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedList {
			tokens: self.tokens,
			elements: self
				.elements
				.into_iter()
				.map(|element| EvaluatedListElement {
					value: element.value.evaluate(context),
					tokens: element.tokens,
				})
				.collect(),
		}
	}
}

impl Spanned for List {
	fn span(&self) -> Span {
		self.tokens.left_bracket.span.to(self.tokens.right_bracket.span)
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedList {
	elements: Vec<EvaluatedListElement>,
	tokens: ListTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedListElement {
	value: Pointer,
	tokens: ListElementTokens,
}
impl Spanned for EvaluatedList {
	fn span(&self) -> Span {
		self.tokens.left_bracket.span.to(self.tokens.right_bracket.span)
	}
}
