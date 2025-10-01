use std::fmt::Debug;

use crate::{
	api::context::Context,
	ast::expressions::{name::Name, Expression, Spanned},
	comptime::{memory::ExpressionPointer, CompileTime},
	diagnostics::Diagnostic,
	io::Io,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	typechecker::Type,
	Span,
};

#[derive(Clone)]
pub struct Parameter {
	pub(crate) name: Name,
	pub(crate) parameter_type: ExpressionPointer,
	pub(crate) span: Span,
}

impl TryParse for Parameter {
	type Output = Parameter;

	fn try_parse<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>) -> Result<Self::Output, Diagnostic> {
		let name = Name::try_parse(tokens, context)?;
		let _ = tokens.pop(TokenType::Colon, context)?;
		let parameter_type = Expression::parse(tokens, context);
		Ok(Parameter {
			span: name.span(context).to(parameter_type.span(context)),
			name,
			parameter_type,
		})
	}
}

impl CompileTime for Parameter {
	type Output = EvaluatedParameter;

	fn evaluate_at_compile_time<System: Io>(self, context: &mut Context<System>) -> Self::Output {
		let evaluated = self.parameter_type.evaluate_to_literal(context);

		EvaluatedParameter {
			name: self.name.clone(),
			parameter_type: Type::Literal(evaluated),
			span: self.span,
		}
	}
}

impl Spanned for Parameter {
	fn span<System: Io>(&self, _context: &Context<System>) -> Span {
		self.span.to_owned()
	}
}

impl Parameter {
	pub(crate) const fn name(&self) -> &Name {
		&self.name
	}
}

impl Debug for Parameter {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:?}: {:?}", self.name, self.parameter_type)
	}
}

#[derive(Debug, Clone)]
pub struct EvaluatedParameter {
	name: Name,
	parameter_type: Type,
	span: Span,
}

impl Spanned for EvaluatedParameter {
	fn span<System: Io>(&self, _context: &Context<System>) -> Span {
		self.span.to_owned()
	}
}

impl EvaluatedParameter {
	pub(crate) const fn parameter_type(&self) -> &Type {
		&self.parameter_type
	}

	pub(crate) const fn name(&self) -> &Name {
		&self.name
	}
}
