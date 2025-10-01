use crate::{
	api::context::Context,
	ast::expressions::{name::Name, Expression},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::Diagnostic,
	interpreter::Runtime,
	io::Io,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::TranspileToC,
	Span,
	Spanned,
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TailStatement {
	pub label: Name,
	pub value: ExpressionPointer,
	pub span: Span,
}

impl TryParse for TailStatement {
	type Output = TailStatement;

	fn try_parse<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>) -> Result<Self::Output, Diagnostic> {
		let label = Name::try_parse(tokens, context)?;

		let _ = tokens.pop(TokenType::KeywordIs, context)?;
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

	fn evaluate_at_compile_time<System: Io>(self, context: &mut Context<System>) -> Self::Output {
		let value = self.value.evaluate_at_compile_time(context);
		TailStatement {
			label: self.label,
			value,
			span: self.span,
		}
	}
}

impl Runtime for TailStatement {
	type Output = TailStatement;

	fn evaluate_at_runtime<System: Io>(self, context: &mut Context<System>) -> Self::Output {
		let value = self.value.evaluate_at_runtime(context);
		TailStatement {
			label: self.label,
			value,
			span: self.span,
		}
	}
}

impl TranspileToC for TailStatement {
	fn to_c<System: Io>(&self, context: &mut Context<System>, _output: Option<String>) -> Result<String, crate::transpiler::TranspileError> {
		let label = self.label.to_c(context, None)?;
		Ok(format!("{}\ngoto label_end_{};", self.value.to_c(context, Some(label.clone()))?, label))
	}
}

impl Spanned for TailStatement {
	fn span<System: Io>(&self, _context: &Context<System>) -> Span {
		self.span
	}
}
