use colored::Colorize as _;

use super::{object::Field, Typed};
use crate::{
	api::{context::context, scope::ScopeId, traits::TryAs as _},
	comptime::{memory::VirtualPointer, CompileTime},
	lexer::{Span, TokenType},
	parser::{
		expressions::{function_declaration::FunctionDeclaration, literal::LiteralConvertible as _, name::Name, operators::PrimaryExpression, Expression, Spanned},
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
	transpiler::TranspileToC,
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

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Diagnostic> {
		let mut expression = PrimaryExpression::try_parse(tokens)?; // There should be no map_err here
		let start = expression.span();
		while tokens.next_is(TokenType::Dot) {
			let _ = tokens.pop(TokenType::Dot)?;
			let right = Name::try_parse(tokens)?;
			let end = right.span();
			expression = Expression::FieldAccess(Self {
				left: Box::new(expression),
				right,
				scope_id: context().scope_data.unique_id(),
				span: start.to(end),
			});
		}

		Ok(expression)
	}
}

impl CompileTime for FieldAccess {
	type Output = Expression;

	fn evaluate_at_compile_time(self) -> anyhow::Result<Self::Output> {
		let left_evaluated = self.left.evaluate_at_compile_time()?;

		// Resolvable at compile-time
		if let Ok(pointer) = left_evaluated.try_as_literal().map(|value| value.address.unwrap()) {
			let literal = pointer.virtual_deref();
			match literal.field_access_type() {
				// Object fields
				FieldAccessType::Normal => {
					let field = literal.get_field(self.right.clone());
					let field = field.ok_or_else(|| {
						anyhow::anyhow!(
							"Attempted to access a the field \"{}\" on an object, but no field with that name exists on that object.",
							self.right.unmangled_name().bold().cyan()
						)
					})?;

					let field_value_literal = field.virtual_deref();
					if field_value_literal.type_name() == &"Function".into() {
						let mut function_declaration = FunctionDeclaration::from_literal(field_value_literal).unwrap();
						function_declaration.set_this_object(left_evaluated);
						context().virtual_memory.replace(field.to_owned(), function_declaration.to_literal());
						Ok(Expression::Pointer(field.to_owned()))
					} else {
						Ok(Expression::Pointer(field))
					}
				},

				// Either fields
				FieldAccessType::Either => {
					let variants = literal.get_internal_field::<Vec<(Name, VirtualPointer)>>("variants").unwrap();
					variants
						.iter()
						.find_map(|(name, value)| (name == &self.right).then_some(Expression::Pointer(value.to_owned())))
						.ok_or_else(|| {
							anyhow::anyhow!(
								"Attempted to access a variant called \"{}\" on an either, but the either has no variant with that name.",
								self.right.unmangled_name().cyan().bold()
							)
						})
				},
			}
		}
		// Not resolvable at compile-time - return the original expression
		else {
			Ok(Expression::FieldAccess(FieldAccess {
				left: Box::new(left_evaluated),
				right: self.right,
				scope_id: self.scope_id,
				span: self.span,
			}))
		}
	}
}

impl TranspileToC for FieldAccess {
	fn to_c(&self) -> anyhow::Result<String> {
		let left = if let Ok(name) = self.left.as_ref().try_as::<Name>() {
			format!("{}_{}", self.left.to_c()?, name.clone().evaluate_at_compile_time()?.try_as_literal()?.address.unwrap())
		} else {
			self.left.to_c()?
		};
		Ok(format!("{}->{}", left, self.right.mangled_name()))
	}
}

impl Spanned for FieldAccess {
	fn span(&self) -> Span {
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
