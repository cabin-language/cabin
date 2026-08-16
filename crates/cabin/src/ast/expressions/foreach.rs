use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::{Expression, block::Block, identifier::Identifier, list::List},
	comptime::{Check as _, Evaluate, storage::Pointer},
	definition::VariableDeclaration,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType,
	traits::TryAs as _,
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct ForLoop {
	/// The name of the variable that acts as the element when iterating. For example, in a loop such as
	/// `for fruit in fruits { ... }`, this would refer to the name `fruit`.
	binding_name: Identifier,

	/// The expression being iterated over. For example, in a loop such as `for fruit in fruits { ... }`, this refers to the
	/// expression `fruits`.
	iterable: Pointer,

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

		let iterable = Expression::try_parse(tokens, context)?;

		let body = Block::try_parse(tokens, context, ScopeType::ForLoop)?;

		let declaration = VariableDeclaration::empty(binding_name.clone(), context);
		if let Err(info) = context.scope.declare_new_variable_from_id(binding_name.clone(), declaration, body.inner_scope_id()) {
			context.add_diagnostic_at(&binding_name, info);
		};

		Ok(ForLoop {
			binding_name,
			body,
			iterable,
			tokens: ForLoopTokens { keyword_for, keyword_in },
		})
	}
}

impl Evaluate for ForLoop {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let iterable = self.iterable.evaluate(context);
		if let Some(expression) = iterable.value(context).clone().as_expression() {
			let Ok(list) = expression.try_as::<List>() else {
				context.add_diagnostic(Diagnostic::at(&iterable, DiagnosticInfo::IterateOverNonList));

				return Expression::ForEachLoop(ForLoop {
					binding_name: self.binding_name,
					iterable,
					body: self.body,
					tokens: self.tokens,
				})
				.store(context);
			};

			let mut elements = list.elements().iter().map(|element: _| element.value().clone());
			let list_is_fully_known = elements.all(|element| element.is_fully_known(context));

			for element in elements {
				context.reassign_variable(&self.binding_name, element.to_owned());

				if list_is_fully_known {
					let _body_value = self.body.clone().evaluate(context);
				} else {
					self.body.check(context);
				}
			}
		}

		Expression::ForEachLoop(ForLoop {
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
