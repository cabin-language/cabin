use std::slice;

use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::misc::parameter::Parameter,
	comptime::Evaluate,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct CompileTimeParameters {
	parameters: Vec<Parameter>,
	tokens: CompileTimeParameterTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct CompileTimeParameterTokens {
	left_angle_bracket: Token,
	right_angle_bracket: Token,
}

impl TryParse for CompileTimeParameters {
	type Output = CompileTimeParameters;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut parameters = Vec::new();
		let left_angle_bracket = tokens.pop(TokenType::LeftAngleBracket, context)?;

		while !tokens.next_is(TokenType::RightAngleBracket) {
			let mut parameter = Parameter::try_parse(tokens, context)?;

			if context.get_variable(parameter.name()).is_some() {
				context.add_diagnostic_at(parameter.name(), DiagnosticInfo::DuplicateVariableDeclaration {
					name: parameter.name().source_identifier().to_owned(),
				});
			}

			let comma = tokens.pop(TokenType::Comma, context).ok();
			let has_comma = comma.is_some();
			parameter.tokens.comma = comma;

			parameters.push(parameter);

			if !has_comma {
				break;
			}
		}

		let right_angle_bracket = tokens.pop(TokenType::RightAngleBracket, context)?;
		Ok(CompileTimeParameters {
			parameters,
			tokens: CompileTimeParameterTokens {
				left_angle_bracket,
				right_angle_bracket,
			},
		})
	}
}

impl Evaluate for CompileTimeParameters {
	type Output = CompileTimeParameters;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		CompileTimeParameters {
			tokens: self.tokens,
			parameters: self.parameters.evaluate(context),
		}
	}
}

impl Spanned for CompileTimeParameters {
	fn span(&self) -> impl SpanLike {
		self.tokens().left_angle_bracket().span().to(self.tokens().right_angle_bracket().span())
	}
}

impl ToCabin for CompileTimeParameters {
	fn to_cabin(&self) -> String {
		format!("<{}>", self.parameters.iter().map(|parameter| parameter.to_cabin()).collect::<Vec<_>>().join(", "))
	}
}

impl CompileTimeParameters {
	pub fn iter(&self) -> slice::Iter<Parameter> {
		self.parameters.iter()
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		self.parameters.iter().all(|parameter| parameter.is_fully_known(context))
	}
}
