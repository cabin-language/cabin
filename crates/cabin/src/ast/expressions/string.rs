use derive_getters::Getters;
// Required because of a bug in `try_as`
use try_as::traits as try_as_traits;

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, binary::EvaluatedBinaryExpression, evaluated::EvaluatedExpression, object::Object},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType, tokenize_string},
	log,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
};

/// A part of a formatted string literal. Each part is either just a regular string value, or an
/// expression that's inserted into the formatted string. The parts are chained together as
/// function calls at parse time, i.e.:
///
/// ```cabin
/// print("Hello {name}!");
/// ```
///
/// becomes:
///
/// ```cabin
/// print("Hello ".plus(name.to_text()).plus("!"));
/// ```
///
/// A formatted string is stored as a `Vec<StringPart>` before being converted into a function call
/// chain such as the one shown above, so the above might be something like:
///
/// ```rust
/// vec![
///     StringPart::Literal("Hello "),
///     StringPart::Expression(name.to_text()),
///     StringPart::Literal("!")
/// ]
/// ```
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

	#[getter(skip)]
	span: Span,
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
		let with_quotes = token.value;
		let mut without_quotes = with_quotes.get(1..with_quotes.len() - 1).unwrap().to_owned();

		let mut parts = vec![StringPart::Literal(LiteralPart { span, value: without_quotes })];
		// let mut builder = without_quotes;
		// while !without_quotes.is_empty() {
		// 	match without_quotes.chars().next().unwrap() {
		// 		'{' => {
		// 			if !builder.is_empty() {
		// 				parts.push(StringPart::Literal(LiteralPart { value: builder, span }));
		// 				builder = String::new();
		// 			}

		// 			// Pop the opening brace
		// 			without_quotes = without_quotes.get(1..without_quotes.len()).unwrap().to_owned();

		// 			// Parse an expression
		// 			let mut expr_tokens = tokenize_string(&without_quotes);
		// 			let expression = Expression::try_parse(&mut expr_tokens, context)?;
		// 			parts.push(StringPart::Expression(expression));

		// 			// Recollect remaining tokens into string
		// 			// TODO: bug here
		// 			// without_quotes = expr_tokens.into_iter().map(|expr_token| expr_token.value).collect();

		// 			// Pop closing brace
		// 			if !without_quotes.starts_with('}') {
		// 				return Err(Diagnostic {
		// 					file: context.file.clone(),
		// 					span: token.span,
		// 					info: DiagnosticInfo::InvalidFormatString(with_quotes),
		// 				});
		// 			}

		// 			without_quotes = without_quotes.get(1..without_quotes.len()).unwrap().to_owned();
		// 		},

		// 		normal_character => {
		// 			without_quotes = without_quotes.get(1..without_quotes.len()).unwrap().to_owned();
		// 			builder.push(normal_character);
		// 		},
		// 	}
		// }

		// if !builder.is_empty() {
		// 	parts.push(StringPart::Literal(LiteralPart { value: builder, span }));
		// }

		Ok(Text { parts, span })
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

		let parts = self.parts.into_iter().map(|part| part.into_expression().evaluate(context)).collect::<Vec<_>>();
		let mut parts = parts.into_iter();

		let mut expression = parts.next().unwrap();
		for part in parts {
			expression = EvaluatedExpression::Binary(EvaluatedBinaryExpression::synthetic(expression, Token::synthetic(TokenType::Plus, "+", Span::none()), part)).store(context);
		}

		expression
	}
}

impl Spanned for Text {
	fn span(&self) -> Span {
		self.span
	}
}

impl Text {
	pub fn literal(literal: LiteralPart) -> Text {
		Text {
			span: literal.span,
			parts: vec![StringPart::Literal(literal)],
		}
	}
}
