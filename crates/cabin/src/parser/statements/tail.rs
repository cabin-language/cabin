use crate::{
	api::{context::context, scope::ScopeType},
	comptime::CompileTime,
	diagnostics::Diagnostic,
	lexer::{Span, TokenType},
	parser::{
		expressions::{name::Name, Expression, Spanned},
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality,
		TryParse,
	},
	transpiler::TranspileToC,
};

#[derive(Debug, Clone)]
pub struct TailStatement {
	pub label: Name,
	pub value: Expression,
	pub span: Span,
}

impl TryParse for TailStatement {
	type Output = TailStatement;

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, Diagnostic> {
		let label = Name::try_parse(tokens)?;

		let _ = tokens.pop(TokenType::KeywordIs)?;
		let value = Expression::parse(tokens);

		Ok(TailStatement {
			span: label.span().to(value.span()),
			label,
			value,
		})
	}
}

impl CompileTime for TailStatement {
	type Output = TailStatement;

	fn evaluate_at_compile_time(self) -> Self::Output {
		let value = self.value.evaluate_at_compile_time();
		TailStatement {
			label: self.label,
			value,
			span: self.span,
		}
	}
}

impl TranspileToC for TailStatement {
	fn to_c(&self) -> anyhow::Result<String> {
		Ok(match context().scope_data.scope_type_of(&self.label)? {
			ScopeType::Function => format!("*return_address = {};\nreturn;", self.value.to_c()?),
			_ => format!("*tail_value = {};\ngoto label_{};", self.value.to_c()?, self.label.to_c()?),
		})
	}
}

impl Spanned for TailStatement {
	fn span(&self) -> Span {
		self.span
	}
}
