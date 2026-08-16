use std::collections::{HashMap, VecDeque};

use derive_getters::Getters;
// Required because of a bug in `try_as`
use try_as::traits as try_as_traits;

use crate::{
	Span,
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, binary::BinaryOperation, function_call::FunctionCall, object::Object},
	comptime::{Dot, Evaluate, storage::Pointer, types::EvaluatedType},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType, tokenize_string},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct Text {
	parts: Vec<StringPart>,
	tokens: TextTokens,
	object: Pointer,
}

#[derive(Debug, Clone, try_as::macros::TryAsRef)]
pub enum StringPart {
	/// A literal string part.
	Literal(LiteralPart),

	/// An interpolated expression string part.
	Expression(Pointer),
}

#[derive(Debug, Clone, Getters)]
pub struct LiteralPart {
	pub span: Span,
	pub value: String,
}

#[derive(Debug, Clone, Getters)]
pub struct TextTokens {
	string: Token,
}

impl TryParse for Text {
	type Output = Text;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::String, context)?;
		let span = &token.span;
		let original = token.value.strip_prefix('"').and_then(|s| s.strip_suffix('"')).unwrap().to_owned();
		let mut without_quotes = original.chars().collect::<VecDeque<char>>();

		let mut parts = Vec::new();
		let mut builder = String::new();

		let span_file = span.file();
		let mut span_start = span.start() + 1;
		while !without_quotes.is_empty() {
			match without_quotes.pop_front().unwrap() {
				'{' => {
					if !builder.is_empty() {
						let part_span = Span::from_start_and_length(span_file, span_start, builder.len());
						let part_end = part_span.end();
						parts.push(StringPart::Literal(LiteralPart {
							span: part_span,
							value: builder.replace(r"\n", "\n"),
						}));
						span_start = part_end + 1;
						builder.clear();
					}

					let mut expr_tokens = tokenize_string(&context.file, &without_quotes.iter().collect::<String>(), span_start);
					let expression = Expression::try_parse(&mut expr_tokens, context)?;
					let length = expression.span().length();
					parts.push(StringPart::Expression(expression));
					span_start += length;

					for _ in 0..length {
						let _ = without_quotes.pop_front();
					}

					if without_quotes.pop_front() != Some('}') {
						return Err(Diagnostic::at(&span, DiagnosticInfo::InvalidFormatString(original)));
					}
					span_start += 1;
				},

				c => builder.push(c),
			}
		}

		if !builder.is_empty() {
			let part_span = Span::from_start_and_length(span_file, span_start, builder.len());
			parts.push(StringPart::Literal(LiteralPart {
				span: part_span,
				value: builder.replace(r"\n", "\n"),
			}));
		}

		Ok(Text {
			parts,
			tokens: TextTokens { string: token },
			object: Object::new(Some(EvaluatedType::builtin("Text", context)), HashMap::new(), Span::none()).store(context),
		})
	}
}

impl Evaluate for Text {
	type Output = Pointer;

	fn evaluate(mut self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating text");

		if self.parts.len() == 1 {
			return match self.parts.remove(0) {
				StringPart::Literal(literal) => Expression::from(Text::literal(literal, context)).store(context),
				StringPart::Expression(expr) => expr.evaluate(context),
			};
		}

		let mut parts = VecDeque::from(self.parts.into_iter().map(|part| part.evalute(context)).collect::<Vec<_>>());
		let mut expression = parts.pop_front().unwrap();

		for part in parts {
			let token = Token::synthetic(TokenType::Plus, "+".to_owned(), part.span().owned());
			expression = FunctionCall::from_binary_expression(expression, BinaryOperation::Add, part, token, context).evaluate(context);
		}

		expression.evaluate(context)
	}
}

impl Spanned for Text {
	fn span(&self) -> impl SpanLike {
		self.tokens().string().span()
	}
}

impl ToCabin for Text {
	fn to_cabin(&self) -> String {
		format!(r#""{}""#, self.parts.to_cabin())
	}
}

impl Text {
	pub fn literal(literal: LiteralPart, context: &mut Context) -> Text {
		Text {
			object: Object::new(Some(EvaluatedType::builtin("Text", context)), HashMap::new(), Span::none()).store(context),
			tokens: TextTokens {
				string: Token::synthetic(TokenType::String, literal.value().to_owned(), literal.span().owned()),
			},
			parts: vec![StringPart::Literal(literal)],
		}
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		self.parts.iter().all(|part| match part {
			StringPart::Literal(_) => true,
			StringPart::Expression(pointer) => pointer.is_fully_known(context),
		})
	}

	pub fn as_literal(&self) -> Option<&String> {
		if self.parts.len() != 1 {
			return None;
		}

		let StringPart::Literal(literal) = self.parts.first().unwrap() else {
			return None;
		};

		Some(&literal.value)
	}
}

impl StringPart {
	pub fn evalute(self, context: &mut Context) -> Pointer {
		match self {
			StringPart::Expression(expression) => expression.evaluate(context),
			StringPart::Literal(literal) => Expression::from(Text::literal(literal, context)).store(context),
		}
	}
}

impl ToCabin for LiteralPart {
	fn to_cabin(&self) -> String {
		self.value.to_owned()
	}
}

impl ToCabin for StringPart {
	fn to_cabin(&self) -> String {
		match self {
			StringPart::Literal(literal_part) => literal_part.to_cabin(),
			StringPart::Expression(expression) => format!("{{{}}}", expression.to_cabin()),
		}
	}
}

impl Dot for Text {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer> {
		self.object.as_object(context).unwrap().clone().dot(context, name)
	}
}
