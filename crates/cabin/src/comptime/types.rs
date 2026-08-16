use crate::{
	Context,
	Span,
	SpanLike,
	Spanned,
	ast::expressions::Expression,
	comptime::{
		Evaluate,
		storage::{Pointer, Storable},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality, TryParse},
	transpiler::ToCabin,
};

#[derive(Debug, Clone)]
pub struct UnevaluatedType {
	inner: Pointer,
	tokens: UnevaluatedTypeTokens,
}

#[derive(Debug, Clone)]
pub struct UnevaluatedTypeTokens {
	editable: Option<Token>,
}

impl TryParse for UnevaluatedType {
	type Output = Self;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let editable = tokens.pop(TokenType::KeywordEditable, context).ok();
		let inner = Expression::try_parse(tokens, context)?;
		Ok(UnevaluatedType {
			inner,
			tokens: UnevaluatedTypeTokens { editable },
		})
	}
}

impl Evaluate for UnevaluatedType {
	type Output = EvaluatedType;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let span = self.span().owned();
		let inner = self.inner.evaluate(context);
		let inner_type = match inner.value(context) {
			Storable::Expression(expression) => match expression {
				Expression::Group(_) => EvaluatedInnerType::Group(inner),
				Expression::Either(_) => EvaluatedInnerType::Either(inner),
				Expression::Action(_) => EvaluatedInnerType::Action(inner),
				_ => {
					context.add_diagnostic_at(&span.owned(), DiagnosticInfo::InvalidType {
						kind: expression.kind_name().to_owned(),
					});
					EvaluatedInnerType::Unknown
				},
			},
			Storable::Object(_) => {
				context.add_diagnostic_at(&span, DiagnosticInfo::InvalidType { kind: "Object".to_owned() });
				EvaluatedInnerType::Unknown
			},
			Storable::Pointer(_) | Storable::Uninitialized => EvaluatedInnerType::Unknown,
		};

		EvaluatedType {
			editable: self.tokens.editable.is_some(),
			inner: inner_type,
			span,
		}
	}
}

impl Spanned for UnevaluatedType {
	fn span(&self) -> impl SpanLike {
		let start = self.tokens.editable.as_ref().map(|token| token.span().owned()).unwrap_or_else(|| self.inner.span().owned());
		let end = self.inner.span();
		start.to(end)
	}
}

impl ToCabin for UnevaluatedType {
	fn to_cabin(&self) -> String {
		todo!()
	}
}

#[derive(Debug, Clone)]
pub struct EvaluatedType {
	pub editable: bool,
	pub inner: EvaluatedInnerType,
	pub span: Span,
}

#[derive(Debug, Clone)]
pub enum EvaluatedInnerType {
	Group(Pointer),
	Either(Pointer),
	Action(Pointer),
	Value(Pointer),
	Unknown,
}

impl Spanned for EvaluatedType {
	fn span(&self) -> impl SpanLike {
		&self.span
	}
}

impl EvaluatedType {
	pub fn is_fully_known(&self, context: &Context) -> bool {
		match &self.inner {
			EvaluatedInnerType::Group(pointer) | EvaluatedInnerType::Action(pointer) | EvaluatedInnerType::Either(pointer) | EvaluatedInnerType::Value(pointer) => {
				pointer.is_fully_known(context)
			},
			EvaluatedInnerType::Unknown => false,
		}
	}

	pub fn builtin(name: &'static str, context: &mut Context) -> EvaluatedType {
		EvaluatedType {
			editable: false,
			inner: EvaluatedInnerType::Group(context.builtin(name)),
			span: Span::none(),
		}
	}
}

#[derive(Debug, Clone)]
pub enum Type {
	Unevaluated(UnevaluatedType),
	Evaluated(EvaluatedType),
}

impl Evaluate for Type {
	type Output = EvaluatedType;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		match self {
			Type::Unevaluated(unevaluated) => unevaluated.evaluate(context),
			Type::Evaluated(evaluated) => evaluated,
		}
	}
}

impl TryParse for Type {
	type Output = Type;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		Ok(Type::Unevaluated(UnevaluatedType::try_parse(tokens, context)?))
	}
}

impl Spanned for Type {
	fn span(&self) -> impl SpanLike {
		match self {
			Type::Unevaluated(unevaluated_type) => unevaluated_type.span().owned(),
			Type::Evaluated(evaluated_type) => evaluated_type.span.owned(),
		}
	}
}

impl ToCabin for Type {
	fn to_cabin(&self) -> String {
		todo!()
	}
}
