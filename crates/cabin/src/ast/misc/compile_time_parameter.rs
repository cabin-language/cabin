use derive_getters::Getters;

use crate::{
	Context,
	Span,
	Spanned,
	ast::misc::parameter::{EvaluatedParameter, Parameter},
	comptime::Evaluate,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
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
	type Output = EvaluatedCompileTimeParameters;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		EvaluatedCompileTimeParameters {
			tokens: self.tokens,
			parameters: self.parameters.evaluate(context),
		}
	}
}

impl Spanned for CompileTimeParameters {
	fn span(&self) -> Span {
		self.tokens.left_angle_bracket.span.to(self.tokens.right_angle_bracket.span)
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedCompileTimeParameters {
	parameters: Vec<EvaluatedParameter>,
	tokens: CompileTimeParameterTokens,
}

impl Spanned for EvaluatedCompileTimeParameters {
	fn span(&self) -> Span {
		self.tokens.left_angle_bracket.span.to(self.tokens.right_angle_bracket.span)
	}
}

impl EvaluatedCompileTimeParameters {
	pub fn iter(&self) -> impl Iterator<Item = &EvaluatedParameter> {
		self.parameters.iter()
	}
}
