use std::collections::HashMap;

use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::expressions::object::Object,
	comptime::{Dot, storage::Pointer, types::EvaluatedType},
	diagnostics::Diagnostic,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct Number {
	pub value: f64,
	object: Pointer,
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
			object: Object::new(Some(EvaluatedType::builtin("Number", context)), HashMap::new(), number.span.clone()).store(context),
			tokens: NumberTokens { number },
		})
	}
}

impl Spanned for Number {
	fn span(&self) -> impl SpanLike {
		self.tokens().number().span()
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

impl Dot for Number {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer> {
		self.object.dot(context, name)
	}
}

impl Number {
	pub fn synthetic(token: Token, context: &mut Context) -> Number {
		Number {
			value: token.value.parse().unwrap(),
			object: Object::new(Some(EvaluatedType::builtin("Number", context)), HashMap::new(), token.span.clone()).store(context),
			tokens: NumberTokens { number: token },
		}
	}
}
