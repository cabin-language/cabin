use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::expressions::{Expression, UnevaluatedExpression, binary::PrimaryExpression, evaluated::EvaluatedExpression, identifier::Identifier},
	comptime::{
		Dot as _,
		Evaluate,
		storage::{Evaluated, Pointer},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone, Getters)]
pub struct FieldAccess {
	pub(crate) left: Pointer<Expression>,
	pub(crate) right: Identifier,
	pub(crate) tokens: FieldAccessTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct FieldAccessTokens {
	pub(crate) operator: Token,
}

impl TryParse for FieldAccess {
	type Output = Pointer<Expression>;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut expression = PrimaryExpression::try_parse(tokens, context)?;

		while tokens.next_is(TokenType::Dot) {
			let operator = tokens.pop(TokenType::Dot, context)?;
			let right = Identifier::try_parse(tokens, context)?;
			expression = UnevaluatedExpression::FieldAccess(Box::new(FieldAccess {
				left: expression,
				right,
				tokens: FieldAccessTokens { operator },
			}))
			.store(context);
		}

		Ok(expression)
	}
}

impl Evaluate for FieldAccess {
	type Output = Pointer<Evaluated>;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating field access");

		let span = self.span().into_owned();
		let left = self.left.evaluate(context);
		let result = left.dot(context, self.right.source_identifier());

		match result {
			Some(field) => {
				if let Evaluated::Expression(EvaluatedExpression::Action(action)) = field.value(context).clone()
					&& action.takes_this(context)
				{
					let mut bound = action.clone();
					bound.this = Some(left);
					return EvaluatedExpression::Action(bound).store(context).into();
				}

				field.with_span(span)
			},
			None => {
				context.add_diagnostic(Diagnostic::at(&self.right, DiagnosticInfo::NoSuchField(self.right.source_identifier().to_owned())));

				EvaluatedExpression::FieldAccess(EvaluatedFieldAccess {
					left,
					right: self.right,
					tokens: self.tokens,
				})
				.store(context)
				.into()
			},
		}
	}
}

impl Spanned for FieldAccess {
	fn span(&self) -> impl SpanLike {
		self.left().span().to(self.right.span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedFieldAccess {
	pub(crate) left: Pointer<Evaluated>,
	pub(crate) right: Identifier,
	pub(crate) tokens: FieldAccessTokens,
}

impl Spanned for EvaluatedFieldAccess {
	fn span(&self) -> impl SpanLike {
		self.left().span().to(self.right.span())
	}
}
