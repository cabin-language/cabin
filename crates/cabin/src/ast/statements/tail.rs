use crate::{
	api::context::Context,
	ast::expressions::{name::Name, Expression},
	comptime::CompileTime,
	diagnostics::Diagnostic,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::TranspileToC,
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct TailStatement {
	pub label: Name,
	pub value: Expression,
	pub span: Span,
}

impl TryParse for TailStatement {
	type Output = TailStatement;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let label = Name::try_parse(tokens, context)?;

		let _ = tokens.pop(TokenType::KeywordIs)?;
		let value = Expression::parse(tokens, context);

		Ok(TailStatement {
			span: label.span(context).to(value.span(context)),
			label,
			value,
		})
	}
}

impl CompileTime for TailStatement {
	type Output = TailStatement;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let value = self.value.evaluate_at_compile_time(context);
		TailStatement {
			label: self.label,
			value,
			span: self.span,
		}
	}
}

impl TranspileToC for TailStatement {
	fn to_c(&self, context: &mut Context, _output: Option<String>) -> Result<String, crate::transpiler::TranspileError> {
		let label = self.label.to_c(context, None)?;
		Ok(format!("{}\ngoto label_end_{};", self.value.to_c(context, Some(label.clone()))?, label))
	}
}

impl Spanned for TailStatement {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}
