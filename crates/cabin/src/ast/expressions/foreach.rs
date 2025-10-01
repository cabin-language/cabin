use super::ExpressionOrPointer;
use crate::{
	api::{context::Context, scope::ScopeId, traits::TryAs as _},
	ast::{
		expressions::{block::Block, name::Name, Expression},
		sugar::list::LiteralList,
	},
	comptime::{memory::ExpressionPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	io::Io,
	lexer::TokenType,
	parser::{Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct ForEachLoop {
	/// The name of the variable that acts as the element when iterating. For example, in a loop such as
	/// `foreach fruit in fruits { ... }`, this would refer to the name `fruit`.
	binding_name: Name,

	/// The expression being iterated over. For example, in a loop such as `foreach fruit in fruits { ... }`, this refers to the
	/// expression `fruits`.
	iterable: ExpressionPointer,

	/// The body of the loop. This is the code that gets run when each iteration of the loop.
	body: Block,

	/// The scope id of for the *inside* of the loop.
	inner_scope_id: ScopeId,

	/// The span of the entire for loop expression. See `Spanned::span` for more details.
	span: Span,
}

impl TryParse for ForEachLoop {
	type Output = ForEachLoop;

	fn try_parse<System: Io>(tokens: &mut TokenQueue, context: &mut Context<System>) -> Result<Self::Output, Diagnostic> {
		let start = tokens.pop(TokenType::KeywordForEach, context)?.span;

		let binding_name = Name::try_parse(tokens, context)?;

		let _ = tokens.pop(TokenType::KeywordIn, context)?;

		let iterable = Expression::parse(tokens, context);

		let body = Block::try_parse(tokens, context)?;

		let end = body.span(context);

		// Add the binding name to scope
		let inner_scope_id = body.inner_scope_id();
		let error = Expression::error(Span::unknown(), context);
		if let Err(error) = context.scope_tree.declare_new_variable_from_id(binding_name.clone(), error, inner_scope_id) {
			context.add_diagnostic(Diagnostic {
				file: context.file.clone(),
				span: binding_name.span(context),
				info: DiagnosticInfo::Error(error),
			});
		}

		Ok(ForEachLoop {
			binding_name,
			body,
			iterable,
			inner_scope_id,
			span: start.to(end),
		})
	}
}

impl CompileTime for ForEachLoop {
	type Output = ExpressionOrPointer;

	fn evaluate_at_compile_time<System: Io>(mut self, context: &mut Context<System>) -> Self::Output {
		self.iterable = self.iterable.evaluate_at_compile_time(context);

		let literal = self.iterable.try_as_literal(context);
		if let Ok(pointer) = literal {
			if !pointer.evaluated_literal(context).is::<LiteralList>() {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					span: self.iterable.span(context),
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::IterateOverNonList)),
				});
				return ExpressionOrPointer::Expression(Expression::ForEachLoop(self));
			}

			let elements = pointer.evaluated_literal(context).try_as::<LiteralList>().cloned().unwrap_or_else(|_| LiteralList::empty());

			for element in &*elements {
				context.scope_tree.reassign_variable_from_id(&self.binding_name, (*element).into(), self.inner_scope_id);
				let _value = self.body.clone().evaluate_at_compile_time(context);
			}
		}

		ExpressionOrPointer::Expression(Expression::ForEachLoop(self))
	}
}

impl Spanned for ForEachLoop {
	fn span<System: Io>(&self, _context: &Context<System>) -> Span {
		self.span
	}
}
