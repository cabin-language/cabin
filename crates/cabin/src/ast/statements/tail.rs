use crate::{
	api::context::Context,
	ast::expressions::{name::Name, Expression},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::Diagnostic,
	interpreter::Runtime,
	io::{IoReader, IoWriter},
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

	fn try_parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Result<Self::Output, Diagnostic> {
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

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
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

	fn evaluate_at_runtime<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		let value = self.value.evaluate_at_runtime(context);
		TailStatement {
			label: self.label,
			value,
			span: self.span,
		}
	}
}

impl TranspileToC for TailStatement {
	fn to_c<Input: IoReader, Output: IoWriter, Error: IoWriter>(
		&self,
		context: &mut Context<Input, Output, Error>,
		_output: Option<String>,
	) -> Result<String, crate::transpiler::TranspileError> {
		let label = self.label.to_c(context, None)?;
		Ok(format!("{}\ngoto label_end_{};", self.value.to_c(context, Some(label.clone()))?, label))
	}
}

impl Spanned for TailStatement {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &Context<Input, Output, Error>) -> Span {
		self.span
	}
}
