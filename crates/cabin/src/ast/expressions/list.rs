use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, object::Object},
	comptime::{
		Dot,
		Evaluate,
		storage::{Evaluated, Pointer},
	},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
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
	value: Pointer<Expression>,
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
		let span = self.span().into_owned();
		let elements = self
			.elements
			.into_iter()
			.map(|element| EvaluatedListElement {
				value: element.value.evaluate(context),
				tokens: element.tokens,
			})
			.collect::<Vec<_>>();

		EvaluatedList {
			tokens: self.tokens,
			object: Object::list(elements.iter().map(|element| element.value().to_owned()).collect(), context, span).store(context),
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

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedList {
	elements: Vec<EvaluatedListElement>,
	tokens: ListTokens,
	object: Pointer<Object>,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedListElement {
	value: Pointer<Evaluated>,
	tokens: ListElementTokens,
}

impl Spanned for EvaluatedList {
	fn span(&self) -> impl SpanLike {
		self.tokens().left_bracket().span().to(self.tokens().right_bracket().span())
	}
}

impl Dot for EvaluatedList {
	fn dot(&self, context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		self.object.dot(context, name)
	}
}
