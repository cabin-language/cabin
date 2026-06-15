use derive_getters::Getters;

use crate::{
	Context,
	Span,
	Spanned,
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct Number {
	value: f64,
	tokens: NumberTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct NumberTokens {
	number: Token,
}

impl TryParse for Number {
	type Output = Number;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let number = tokens.pop(TokenType::Number, context)?;

		Ok(Number {
			value: number.value.parse().unwrap(),
			tokens: NumberTokens { number },
		})
	}
}

impl Spanned for Number {
	fn span(&self) -> Span {
		self.tokens.number.span
	}
}

impl ToCabin for Number {
	fn to_cabin(&self) -> String {
		let x = self.value;

		// integer
		if (x - x.round()).abs() < f64::EPSILON {
			return format!("{}", x.round() as i64);
		}

		// format to 10 places
		let mut string = format!("{x:.10}");

		// remove trailing 0 / decimal (in case of floating point errors)
		string = string.trim_end_matches("0").trim_end_matches(".").to_owned();
		string
	}
}
