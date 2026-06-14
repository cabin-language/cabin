use std::collections::VecDeque;

use derive_getters::Getters;
// Required because of a bug in `try_as`
use try_as::traits as try_as_traits;

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, binary::BinaryExpression, object::Object},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType, tokenize_string},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, try_as::macros::TryAsRef)]
pub enum StringPart {
	/// A literal string part.
	Literal(LiteralPart),

	/// An interpolated expression string part.
	Expression(Expression),
}

impl StringPart {
	pub fn into_expression(self) -> Expression {
		match self {
			StringPart::Expression(expression) => expression,
			StringPart::Literal(literal) => Expression::Text(Text::literal(literal)),
		}
	}
}

#[derive(Debug, Clone, Getters)]
pub struct LiteralPart {
	span: Span,
	value: String,
}

#[derive(Debug, Clone, Getters)]
pub struct Text {
	parts: Vec<StringPart>,
	tokens: TextTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct TextTokens {
	string: Token,
}

impl TryParse for Text {
	type Output = Text;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::String, context)?;
		let span = token.span;
		let original = token.value.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap().to_owned();
		let mut without_quotes = original.chars().collect::<VecDeque<char>>();

		let mut parts = Vec::new();
		let mut builder = String::new();

		let mut span_start = span.start;
		while !without_quotes.is_empty() {
			match without_quotes.pop_front().unwrap() {
				'{' => {
					if !builder.is_empty() {
						let part_span = Span::new(span_start, builder.len());
						parts.push(StringPart::Literal(LiteralPart {
							span: part_span,
							value: builder.replace(r"\n", "\n"),
						}));
						span_start = part_span.end() + 1;
						builder.clear();
					}

					let mut expr_tokens = tokenize_string(&without_quotes.iter().collect::<String>());
					let expression = Expression::try_parse(&mut expr_tokens, context)?;
					let length = expression.span().length();
					parts.push(StringPart::Expression(expression));
					span_start += length;

					for _ in 0..length {
						let _ = without_quotes.pop_front();
					}

					if without_quotes.pop_front() != Some('}') {
						return Err(Diagnostic {
							info: DiagnosticInfo::InvalidFormatString(original),
							file: context.file.clone(),
							span,
						});
					}
				},

				c => builder.push(c),
			}
		}

		if !builder.is_empty() {
			let part_span = Span::new(span_start, builder.len());
			parts.push(StringPart::Literal(LiteralPart {
				span: part_span,
				value: builder.replace(r"\n", "\n"),
			}));
		}

		Ok(Text {
			parts,
			tokens: TextTokens { string: token },
		})
	}
}

impl Evaluate for Text {
	type Output = Pointer;

	fn evaluate(mut self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating text");

		if self.parts.len() == 1 {
			return match self.parts.remove(0) {
				StringPart::Literal(literal) => Object::text(literal.value, context, literal.span).store(context),
				StringPart::Expression(expr) => expr.evaluate(context),
			};
		}

		let mut parts = self.parts.into_iter().map(|part| part.into_expression());
		let mut expression = parts.next().unwrap();

		for part in parts {
			expression = Expression::Binary(Box::new(BinaryExpression::synthetic(expression, Token::synthetic(TokenType::Plus, "+", part.span()), part)));
		}

		expression.evaluate(context)
	}
}

impl Spanned for Text {
	fn span(&self) -> Span {
		self.tokens.string.span
	}
}

impl Text {
	pub fn literal(literal: LiteralPart) -> Text {
		Text {
			tokens: TextTokens {
				string: Token::synthetic(TokenType::String, &literal.value, literal.span),
			},
			parts: vec![StringPart::Literal(literal)],
		}
	}
}
