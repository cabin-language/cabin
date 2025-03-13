use std::collections::VecDeque;

// Required because of a bug in `try_as`
use try_as::traits as try_as_traits;

use crate::{
	api::{context::Context, traits::TryAs as _},
	ast::expressions::{
		field_access::{Dot, FieldAccess},
		function_call::FunctionCall,
		literal::EvaluatedLiteral,
		name::Name,
		Expression,
	},
	comptime::memory::ExpressionPointer,
	diagnostics::{Diagnostic, DiagnosticInfo},
	io::{IoReader, IoWriter},
	lexer::{tokenize_string, Token, TokenType},
	parser::{Parse as _, ParseError, TokenQueue, TokenQueueFunctionality as _, TryParse},
	Span,
	Spanned,
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
#[derive(Debug, try_as::macros::TryAsRef)]
pub(crate) enum StringPart {
	/// A literal string part.
	Literal(CabinString),

	/// An interpolated expression string part.
	Expression(ExpressionPointer),
}

impl StringPart {
	pub(crate) fn into_expression<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> ExpressionPointer {
		match self {
			StringPart::Expression(expression) => expression,
			StringPart::Literal(literal) => Expression::EvaluatedLiteral(EvaluatedLiteral::String(literal)).store_in_memory(context),
		}
	}
}

/// A wrapper for implementing `Parse` for parsing string literals. In Cabin, all strings are
/// formatted strings by default, so they require special logic for parsing.
#[derive(Debug, Clone)]
pub struct CabinString {
	pub(crate) span: Span,
	pub(crate) value: String,
}

impl TryParse for CabinString {
	type Output = ExpressionPointer;

	fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Result<Self::Output, Diagnostic> {
		let token = tokens.pop(TokenType::String, context)?;
		let span = token.span;
		let with_quotes = token.value;
		let mut without_quotes = with_quotes.get(1..with_quotes.len() - 1).unwrap().to_owned();

		let mut parts = Vec::new();
		let mut builder = String::new();
		while !without_quotes.is_empty() {
			match without_quotes.chars().next().unwrap() {
				'{' => {
					if !builder.is_empty() {
						parts.push(StringPart::Literal(CabinString { value: builder, span }));
						builder = String::new();
					}
					// Pop the opening brace
					without_quotes = without_quotes.get(1..without_quotes.len()).unwrap().to_owned();

					// Parse an expression
					let mut tokens = tokenize_string(&without_quotes);
					let expression = Expression::parse(&mut tokens, context);
					parts.push(StringPart::Expression(expression));

					// Recollect remaining tokens into string
					without_quotes = tokens.into_iter().map(|token| token.value).collect();

					// Pop closing brace
					if without_quotes.chars().next().unwrap() != '}' {
						return Err(Diagnostic {
							file: context.file.clone(),
							span: token.span,
							info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::InvalidFormatString(with_quotes))),
						});
					}
					without_quotes = without_quotes.get(1..without_quotes.len()).unwrap().to_owned();
				},
				normal_character => {
					without_quotes = without_quotes.get(1..without_quotes.len()).unwrap().to_owned();
					builder.push(normal_character);
				},
			}
		}
		if !builder.is_empty() {
			parts.push(StringPart::Literal(CabinString { value: builder, span }));
		}

		if parts.iter().all(|part| matches!(part, StringPart::Literal(_))) {
			return Ok(Expression::EvaluatedLiteral(EvaluatedLiteral::String(CabinString {
				value: parts.into_iter().map(|part| part.try_as::<CabinString>().unwrap().value.clone()).collect::<String>(),
				span,
			}))
			.store_in_memory(context));
		}

		// Composite into function call, i.e., "hello {name}!" becomes
		// "hello ".plus(name.to_text()).plus("!")
		let mut parts = VecDeque::from(parts);
		let mut left = parts.pop_front().unwrap().into_expression(context);
		for part in parts {
			let mut right = part.into_expression(context);
			right = Expression::FunctionCall(FunctionCall::basic(
				Expression::FieldAccess(FieldAccess::new(right, "to_text".into(), context.scope_tree.unique_id(), span)).store_in_memory(context),
				context,
			))
			.store_in_memory(context);
			left = Expression::FunctionCall(FunctionCall::from_binary_operation(context, left, right, Token {
				token_type: TokenType::Plus,
				value: "+".to_owned(),
				span,
			})?)
			.store_in_memory(context);
		}

		Ok(left)
	}
}

impl Spanned for CabinString {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span
	}
}

impl Dot for CabinString {
	fn dot<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, name: &Name, context: &mut Context<Input, Output, Error>) -> ExpressionPointer {
		match name.unmangled_name() {
			_ => ExpressionPointer::ERROR,
		}
	}
}
