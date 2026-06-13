use std::fmt::Debug;

use derive_getters::Getters;

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, identifier::Identifier},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct Parameter {
	name: Identifier,
	parameter_type: Expression,
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
		let parameter_type = Expression::try_parse(tokens, context)?;

		Ok(Parameter {
			name,
			parameter_type,
			tokens: ParameterTokens { colon, comma: None },
		})
	}
}

impl Evaluate for Parameter {
	type Output = EvaluatedParameter;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedParameter {
			name: self.name.clone(),
			parameter_type: self.parameter_type.evaluate(context),
		}
	}
}

impl Spanned for Parameter {
	fn span(&self) -> Span {
		self.name.span().to(self.parameter_type.span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedParameter {
	name: Identifier,
	parameter_type: Pointer,
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

			if context.scope.get_variable(parameter.name().source_identifier()).is_some() {
				context.add_diagnostic(Diagnostic {
					span: parameter.name().span(),
					file: context.file.clone(),
					info: DiagnosticInfo::DuplicateVariableDeclaration {
						name: parameter.name().source_identifier().to_owned(),
					},
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

impl Spanned for Parameters {
	fn span(&self) -> Span {
		self.tokens.left_parenthesis.span.to(self.tokens.right_parenthesis.span)
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedParameters {
	parameters: Vec<EvaluatedParameter>,
	tokens: ParametersTokens,
}

impl Evaluate for Parameters {
	type Output = EvaluatedParameters;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedParameters {
			tokens: self.tokens,
			parameters: self.parameters.evaluate(context),
		}
	}
}

impl Spanned for EvaluatedParameters {
	fn span(&self) -> Span {
		self.tokens.left_parenthesis.span.to(self.tokens.right_parenthesis.span)
	}
}

impl EvaluatedParameters {
	pub fn iter(&self) -> impl Iterator<Item = &EvaluatedParameter> {
		self.parameters.iter()
	}
}
