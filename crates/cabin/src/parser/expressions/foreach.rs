use super::Spanned;
use crate::{
	api::{context::Context, scope::ScopeId, traits::TryAs as _},
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Span, TokenType},
	parser::{
		expressions::{block::Block, name::Name, Expression},
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
};

#[derive(Debug, Clone)]
pub struct ForEachLoop {
	/// The name of the variable that acts as the element when iterating. For example, in a loop such as
	/// `foreach fruit in fruits { ... }`, this would refer to the name `fruit`.
	binding_name: Name,

	/// The expression being iterated over. For example, in a loop such as `foreach fruit in fruits { ... }`, this refers to the
	/// expression `fruits`.
	iterable: Box<Expression>,

	/// The body of the loop. This is the code that gets run when each iteration of the loop.
	body: Box<Expression>,

	/// The scope id of for the *inside* of the loop.
	inner_scope_id: ScopeId,

	/// The span of the entire for loop expression. See `Spanned::span` for more details.
	span: Span,
}

impl TryParse for ForEachLoop {
	type Output = ForEachLoop;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordForEach)?.span;

		let binding_name = Name::try_parse(tokens, context)?;

		let _ = tokens.pop(TokenType::KeywordIn)?;

		let iterable = Box::new(Expression::parse(tokens, context));

		let body = Block::try_parse(tokens, context)?;

		let end = body.span(context);

		// Add the binding name to scope
		let inner_scope_id = body.inner_scope_id();
		if let Err(error) = context
			.scope_data
			.declare_new_variable_from_id(binding_name.clone(), Expression::ErrorExpression(Span::unknown()), inner_scope_id)
		{
			context.add_diagnostic(Diagnostic {
				span: binding_name.span(context),
				info: DiagnosticInfo::Error(error),
			});
		}

		Ok(ForEachLoop {
			binding_name,
			body: Box::new(Expression::Block(body)),
			iterable,
			inner_scope_id,
			span: start.to(end),
		})
	}
}

impl CompileTime for ForEachLoop {
	type Output = Expression;

	fn evaluate_at_compile_time(mut self, context: &mut Context) -> Self::Output {
		self.iterable = Box::new(self.iterable.evaluate_at_compile_time(context));

		let default = Vec::new();
		let literal = self.iterable.as_ref().try_as::<VirtualPointer>();
		if let Ok(pointer) = literal {
			if !pointer.is_list(context) {
				context.add_diagnostic(Diagnostic {
					span: self.iterable.span(context),
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::IterateOverNonList)),
				});
				return Expression::ForEachLoop(self);
			}

			let elements = pointer
				.virtual_deref(context)
				.try_as::<Vec<Expression>>()
				.unwrap_or_else(|_| &default)
				.to_owned()
				.into_iter()
				.map(|element| element.evaluate_at_compile_time(context))
				.collect::<Vec<_>>();

			for element in elements {
				context.scope_data.reassign_variable_from_id(&self.binding_name, element.clone(), self.inner_scope_id);
				let value = self.body.clone().evaluate_at_compile_time(context);
				if value.is_pointer() {
					return value;
				}
			}
		}

		Expression::ForEachLoop(self)
	}
}

impl Spanned for ForEachLoop {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}
