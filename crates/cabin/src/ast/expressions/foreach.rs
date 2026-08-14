use derive_getters::Getters;

use crate::{
	SpanLike, Spanned, api::context::Context, ast::expressions::{EvaluatedExpression, UnevaluatedExpression, block::Block, identifier::Identifier}, comptime::{Check as _, Evaluate, storage::ExpressionRef, types::Type}, definition::UnevaluatedVariableDeclaration, diagnostics::{Diagnostic, DiagnosticInfo}, lexer::{Token, TokenType}, parser::{TokenQueue, TokenQueueFunctionality as _, TryParse}, scope::ScopeType, transpiler::ToCabin
};

#[derive(Debug, Clone, Getters)]
pub struct ForLoop {
	/// The name of the variable that acts as the element when iterating. For example, in a loop such as
	/// `for fruit in fruits { ... }`, this would refer to the name `fruit`.
	binding_name: Identifier,

	/// The expression being iterated over. For example, in a loop such as `for fruit in fruits { ... }`, this refers to the
	/// expression `fruits`.
	iterable: UnevaluatedExpression,

	/// The body of the loop. This is the code that gets run when each iteration of the loop.
	body: Block,

	tokens: ForLoopTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ForLoopTokens {
	keyword_for: Token,
	keyword_in: Token,
}

impl TryParse for ForLoop {
	type Output = ForLoop;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_for = tokens.pop(TokenType::KeywordForEach, context)?;

		let binding_name = Identifier::try_parse(tokens, context)?;

		let keyword_in = tokens.pop(TokenType::KeywordIn, context)?;

		let iterable = UnevaluatedExpression::try_parse(tokens, context)?;

		let body = Block::try_parse(tokens, context, ScopeType::ForLoop)?;

		let declaration = UnevaluatedVariableDeclaration::new(binding_name.clone(), Type::new(), context.none_expr())
		context.scope.declare_new_variable_from_id(binding_name.clone(), declaration.into(), body.inner_scope_id());

		Ok(ForLoop {
			binding_name,
			body,
			iterable,
			tokens: ForLoopTokens { keyword_for, keyword_in },
		})
	}
}

impl Evaluate for ForLoop {
	type Output = ExpressionRef;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let iterable = self.iterable.evaluate(context);
		if let Some(object) = iterable.value(context).clone().as_object() {
			let Some(elements) = object.list_elements() else {
				context.add_diagnostic(Diagnostic::at(&iterable, DiagnosticInfo::IterateOverNonList));

				return EvaluatedExpression::ForLoop(EvaluatedForLoop {
					binding_name: self.binding_name,
					iterable,
					body: self.body,
					tokens: self.tokens,
				})
				.store(context);
			};

			let list_is_fully_known = elements.iter().all(|element| element.is_fully_known(context));

			for element in elements {
				context.scope.reassign_variable_from_id(&self.binding_name, element.to_owned(), self.body.inner_scope_id());

				if list_is_fully_known {
					let _body_value = self.body.clone().evaluate(context);
				} else {
					self.body.check(context);
				}
			}
		}

		EvaluatedExpression::ForLoop(EvaluatedForLoop {
			binding_name: self.binding_name,
			iterable,
			body: self.body,
			tokens: self.tokens,
		})
		.store(context)
	}
}

impl Spanned for ForLoop {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_for().span().to(self.body.span())
	}
}

impl ToCabin for ForLoop {
	fn to_cabin(&self) -> String {
		let mut code = "for ".to_owned();
		code += &self.binding_name.to_cabin();
		code += " in ";
		code += &self.iterable.to_cabin();
		code += " ";
		code += &self.body.to_cabin();
		code
	}
}

impl Spanned for EvaluatedForLoop {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_for().span().to(self.body.span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedForLoop {
	/// The name of the variable that acts as the element when iterating. For example, in a loop such as
	/// `for fruit in fruits { ... }`, this would refer to the name `fruit`.
	binding_name: Identifier,

	/// The expression being iterated over. For example, in a loop such as `for fruit in fruits { ... }`, this refers to the
	/// expression `fruits`.
	iterable: ExpressionRef,

	/// The body of the loop. This is the code that gets run when each iteration of the loop.
	body: Block,

	tokens: ForLoopTokens,
}
