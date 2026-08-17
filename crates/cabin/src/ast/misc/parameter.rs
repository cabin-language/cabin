use std::{fmt::Debug, slice};

use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::identifier::Identifier,
	comptime::{Evaluate, types::Type},
	context,
	definition::VariableDeclaration,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct Parameter {
	name: Identifier,
	parameter_type: Type,
	pub(crate) tokens: ParameterTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ParameterTokens {
	colon: Token,
	pub(crate) comma: Option<Token>,
}

impl TryParse for Parameter {
	type Output = Parameter;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let name = Identifier::try_parse(tokens, context)?;
		let colon = tokens.pop(TokenType::Colon, context)?;
		let parameter_type = Type::try_parse(tokens, context)?;

		Ok(Parameter {
			name,
			parameter_type,
			tokens: ParameterTokens { colon, comma: None },
		})
	}
}

impl Evaluate for Parameter {
	type Output = Parameter;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		Parameter {
			name: self.name.clone(),
			tokens: self.tokens.clone(),
			parameter_type: Type::Evaluated(self.parameter_type.evaluate(context)),
		}
	}
}

impl Spanned for Parameter {
	fn span(&self) -> impl SpanLike {
		self.name().span().to(self.parameter_type.span())
	}
}

impl ToCabin for Parameter {
	fn to_cabin(&self) -> String {
		format!("{}: {}", self.name.to_cabin(), self.parameter_type.to_cabin())
	}
}

impl Parameter {
	pub fn as_declaration(&self, context: &mut Context) -> VariableDeclaration {
		VariableDeclaration::with_type(self.name().to_owned(), self.parameter_type().to_owned(), context)
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		match &self.parameter_type {
			Type::Unevaluated(unevaluated_type) => false,
			Type::Evaluated(evaluated_type) => evaluated_type.is_fully_known(context),
		}
	}
}

#[derive(Debug, Clone, Getters)]
pub struct Parameters {
	parameters: Vec<Parameter>,
	tokens: ParametersTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ParametersTokens {
	left_parenthesis: Token,
	right_parenthesis: Token,
}

impl TryParse for Parameters {
	type Output = Parameters;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut parameters = Vec::new();
		let left_angle_bracket = tokens.pop(TokenType::LeftParenthesis, context)?;
		while !tokens.next_is(TokenType::RightAngleBracket) {
			let mut parameter = Parameter::try_parse(tokens, context)?;

			if context.variable_exists(parameter.name()) {
				context.add_diagnostic_at(parameter.name(), DiagnosticInfo::DuplicateVariableDeclaration {
					name: parameter.name().source_text().to_owned(),
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

		let right_angle_bracket = tokens.pop(TokenType::RightParenthesis, context)?;

		Ok(Parameters {
			parameters,
			tokens: ParametersTokens {
				left_parenthesis: left_angle_bracket,
				right_parenthesis: right_angle_bracket,
			},
		})
	}
}

impl Parameters {
	pub fn remove_first(&mut self) {
		let _ = self.parameters.remove(0);
	}
}

impl Spanned for Parameters {
	fn span(&self) -> impl SpanLike {
		self.tokens().left_parenthesis().span().to(self.tokens().right_parenthesis().span())
	}
}

impl ToCabin for Parameters {
	fn to_cabin(&self) -> String {
		format!("({})", self.parameters.iter().map(|parameter| parameter.to_cabin()).collect::<Vec<_>>().join(", "))
	}
}

impl Evaluate for Parameters {
	type Output = Parameters;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		Parameters {
			tokens: self.tokens,
			parameters: self.parameters.evaluate(context),
		}
	}
}

impl Parameters {
	pub fn first(&self) -> Option<&Parameter> {
		self.parameters.first()
	}

	pub fn len(&self) -> usize {
		self.parameters.len()
	}

	pub fn iter(&self) -> slice::Iter<Parameter> {
		self.parameters.iter()
	}
}
