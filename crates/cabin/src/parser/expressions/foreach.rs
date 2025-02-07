use super::Spanned;
use crate::{
	api::{context::context, scope::ScopeId, traits::TryAs as _},
	comptime::CompileTime,
	lexer::{Span, TokenType},
	parser::{
		expressions::{block::Block, name::Name, Expression},
		Parse as _,
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
	transpiler::TranspileToC,
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

	fn try_parse(tokens: &mut TokenQueue) -> Result<Self::Output, crate::Diagnostic> {
		let start = tokens.pop(TokenType::KeywordForEach)?.span;

		let binding_name = Name::try_parse(tokens)?;

		let _ = tokens.pop(TokenType::KeywordIn)?;

		let iterable = Box::new(Expression::parse(tokens));

		let body = Block::try_parse(tokens)?;

		let end = body.span();

		// Add the binding name to scope
		let inner_scope_id = body.inner_scope_id();
		context()
			.scope_data
			.declare_new_variable_from_id(binding_name.clone(), Expression::ErrorExpression(Span::unknown()), inner_scope_id)?;

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

	fn evaluate_at_compile_time(mut self) -> anyhow::Result<Self::Output> {
		self.iterable = Box::new(self.iterable.evaluate_at_compile_time()?);
		if let Ok(literal) = self.iterable.try_as_literal() {
			let elements = literal
				.try_as::<Vec<Expression>>()?
				.to_owned()
				.into_iter()
				.map(|element| element.evaluate_at_compile_time())
				.collect::<anyhow::Result<Vec<_>>>()?;
			for element in elements {
				context().scope_data.reassign_variable_from_id(&self.binding_name, element.clone(), self.inner_scope_id)?;
				let value = self.body.clone().evaluate_at_compile_time()?;
				if value.is_pointer() {
					return Ok(value);
				}
			}
		}

		Ok(Expression::ForEachLoop(self))
	}
}

impl TranspileToC for ForEachLoop {
	fn to_c(&self) -> anyhow::Result<String> {
		Ok(format!(
			"({{\n\tlet elements = {};\n\tfor (int index = 0; index < elements->length(); index++) {{\n\t{}\n\t}}\n}})",
			self.iterable.to_c()?,
			self.body.to_c()?.lines().map(|line| format!("\t\t{line}")).collect::<Vec<_>>().join("\n")
		))
	}
}

impl Spanned for ForEachLoop {
	fn span(&self) -> Span {
		self.span
	}
}
