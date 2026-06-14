use derive_getters::Getters;

use crate::{
	Context,
	Span,
	Spanned,
	ast::expressions::{Expression, binary::PrimaryExpression, evaluated::EvaluatedExpression, identifier::Identifier},
	comptime::{
		Evaluate,
		memory::{Pointer, Storable},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
};

#[derive(Debug, Clone)]
pub struct FieldAccess {
	pub(crate) left: Expression,
	pub(crate) right: Identifier,
	pub(crate) tokens: FieldAccessTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct FieldAccessTokens {
	pub(crate) operator: Token,
}

impl TryParse for FieldAccess {
	type Output = Expression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut expression = PrimaryExpression::try_parse(tokens, context)?;

		while tokens.next_is(TokenType::Dot) {
			let operator = tokens.pop(TokenType::Dot, context)?;
			let right = Identifier::try_parse(tokens, context)?;
			expression = Expression::FieldAccess(Box::new(FieldAccess {
				left: expression,
				right,
				tokens: FieldAccessTokens { operator },
			}));
		}

		Ok(expression)
	}
}

impl Evaluate for FieldAccess {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating field access");
		let span = self.span();
		let left = self.left.evaluate(context);

		if let Some(object) = left.value(context).as_object() {
			let field_value = object.get_field_value(self.right.source_identifier());

			if let Some(pointer) = field_value {
				if let Storable::Expression(EvaluatedExpression::Action(action)) = pointer.value(context)
					&& action.takes_this(context)
				{
					let mut bound = action.clone();
					bound.this = Some(left);
					return EvaluatedExpression::Action(bound).store(context);
				}

				return pointer.with_span(span);
			}

			for extension in context.extensions_for(*object.group()) {
				if let Some(field) = extension.fields().iter().find(|field| field.name() == &self.right) {
					if let Storable::Expression(EvaluatedExpression::Action(action)) = field.value().value(context)
						&& action.takes_this(context)
					{
						let mut bound = action.clone();
						bound.this = Some(left);
						return EvaluatedExpression::Action(bound).store(context);
					}

					return field.value().with_span(span);
				}
			}

			context.add_diagnostic(Diagnostic {
				span: self.right.span(),
				info: DiagnosticInfo::NoSuchField(self.right.source_identifier().to_owned()),
				file: context.file.clone(),
			});
		}

		EvaluatedExpression::FieldAccess(EvaluatedFieldAccess {
			left,
			right: self.right,
			tokens: self.tokens,
		})
		.store(context)
	}
}

impl Spanned for FieldAccess {
	fn span(&self) -> Span {
		self.left.span().to(self.right.span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedFieldAccess {
	pub(crate) left: Pointer,
	pub(crate) right: Identifier,
	pub(crate) tokens: FieldAccessTokens,
}

impl Spanned for EvaluatedFieldAccess {
	fn span(&self) -> Span {
		self.left.span().to(self.right.span())
	}
}
