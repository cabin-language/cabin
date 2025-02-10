use crate::{
	api::{context::Context, scope::ScopeId},
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Span, TokenType},
	parser::{
		expressions::{function_declaration::FunctionDeclaration, literal::LiteralConvertible as _, name::Name, operators::PrimaryExpression, Expression, Spanned},
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
};

/// A type describing how fields are accessed on this type of objects via the dot operator.
/// For example, on a normal object, the dot operator just gets a field with the given name,
/// but for `eithers`, it indexes into the either's variants and finds the one with the given
/// name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldAccessType {
	Normal,
	Either,
}

#[derive(Debug, Clone)]
pub struct FieldAccess {
	left: Box<Expression>,
	right: Name,
	scope_id: ScopeId,
	span: Span,
}

impl TryParse for FieldAccess {
	type Output = Expression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let mut expression = PrimaryExpression::try_parse(tokens, context)?;
		let start = expression.span(context);
		while tokens.next_is(TokenType::Dot) {
			let _ = tokens.pop(TokenType::Dot)?;
			let right = Name::try_parse(tokens, context)?;
			let end = right.span(context);
			expression = Expression::FieldAccess(Self {
				left: Box::new(expression),
				right,
				scope_id: context.scope_data.unique_id(),
				span: start.to(end),
			});
		}

		Ok(expression)
	}
}

impl CompileTime for FieldAccess {
	type Output = Expression;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let span = self.span(context);
		let left_evaluated = self.left.evaluate_at_compile_time(context);

		// Resolvable at compile-time
		let pointer = left_evaluated.try_as_literal(context).address.unwrap();
		if pointer != VirtualPointer::ERROR {
			let literal = pointer.virtual_deref(context);
			match literal.field_access_type() {
				// Object fields
				FieldAccessType::Normal => {
					let field = literal.get_field(self.right.clone());
					let field = field.unwrap_or_else(|| {
						context.add_diagnostic(Diagnostic {
							span: self.right.span(context),
							info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::FieldNotFound(self.right.unmangled_name().to_owned()))),
						});
						VirtualPointer::ERROR
					});

					let field_value_literal = field.virtual_deref(context);
					if field_value_literal.type_name() == &"Function".into() {
						let mut function_declaration = FunctionDeclaration::from_literal(field_value_literal).unwrap();
						function_declaration.set_this_object(left_evaluated);
						context.virtual_memory.replace(field.to_owned(), function_declaration.to_literal());
						Expression::Pointer(field.to_owned())
					} else {
						Expression::Pointer(field)
					}
				},

				// Either fields
				FieldAccessType::Either => {
					let variants = literal.get_internal_field::<Vec<(Name, VirtualPointer)>>("variants").unwrap();
					variants
						.iter()
						.find_map(|(name, value)| (name == &self.right).then_some(Expression::Pointer(value.to_owned())))
						.unwrap_or_else(|| {
							context.add_diagnostic(Diagnostic {
								span: self.right.span(context),
								info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::FieldNotFound(self.right.unmangled_name().to_owned()))),
							});
							Expression::ErrorExpression(span)
						})
				},
			}
		}
		// Not resolvable at compile-time - return the original expression
		else {
			Expression::FieldAccess(FieldAccess {
				left: Box::new(left_evaluated),
				right: self.right,
				scope_id: self.scope_id,
				span: self.span,
			})
		}
	}
}

impl Spanned for FieldAccess {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl FieldAccess {
	pub fn new(left: Expression, right: Name, scope_id: ScopeId, span: Span) -> FieldAccess {
		FieldAccess {
			left: Box::new(left),
			right,
			scope_id,
			span,
		}
	}
}
