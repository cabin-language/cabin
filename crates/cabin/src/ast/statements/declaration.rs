use convert_case::{Case, Casing as _};
use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::{
		expressions::{Expression, identifier::Identifier},
		misc::tag::TagList,
		statements::Statement,
	},
	comptime::{
		Evaluate,
		storage::{Pointer, Storable},
	},
	definition::VariableDeclaration,
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeRef,
};

#[derive(Debug, Clone, Getters)]
pub struct Declaration {
	name: Identifier,
	explicit_type: Option<Pointer>,
	value: Pointer,
	tokens: DeclarationTokens,
	scope: ScopeRef,
}

#[derive(Debug, Clone, Getters)]
pub struct DeclarationTokens {
	keyword_visible: Option<Token>,
	keyword_editable: Option<Token>,
	keyword_let: Token,
	equal: Token,
	semicolon: Token,
	colon: Option<Token>,
}

impl TryParse for Declaration {
	type Output = Statement;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// Tags
		let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context)?);

		if tags.is_some() && !tokens.next_is(TokenType::KeywordLet) {
			let expression = Expression::try_parse(tokens, context)?;
			let _ = tokens.pop(TokenType::Semicolon, context)?;
			return Ok(Statement::Expression(expression));
		}

		// let
		let keyword_let = tokens.pop(TokenType::KeywordLet, context)?;

		let keyword_visible = if_then_some!(tokens.next_is(TokenType::KeywordVisible), tokens.pop(TokenType::KeywordVisible, context).unwrap());
		let keyword_editable = if_then_some!(tokens.next_is(TokenType::KeywordEditable), tokens.pop(TokenType::KeywordEditable, context).unwrap());

		// name
		let name = Identifier::try_parse(tokens, context)?;
		context.add_to_path(name.source_text());

		let location = context.storage.calloc();
		context.declare_new_variable(VariableDeclaration::with_value(name.clone(), location));

		// type
		let (colon, explicit_type) = if_then_some!(tokens.next_is(TokenType::Colon), {
			(tokens.pop(TokenType::Colon, context).unwrap(), Expression::try_parse(tokens, context)?)
		})
		.unzip();

		// Value
		let equal = tokens.pop(TokenType::Equal, context)?;

		let value = Expression::try_parse(tokens, context)?;

		// add the name declaration to the scope
		context.reassign_variable(&name, value.clone());

		let semicolon = tokens.pop(TokenType::Semicolon, context)?;

		context.pop_from_path();

		// return the declaration
		Ok(Statement::Declaration(Declaration {
			name,
			value,
			explicit_type,
			scope: context.scope.get_ref(),
			tokens: DeclarationTokens {
				colon,
				keyword_visible,
				keyword_editable,
				keyword_let,
				equal,
				semicolon,
			},
		}))
	}
}

impl Evaluate for Declaration {
	type Output = Declaration;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating declaration for \"{}\"", self.name.source_text());
		let evaluated = self.value.evaluate(context);

		if let Storable::Expression(value) = evaluated.value(context) {
			match value {
				Expression::Group(_) | Expression::Either(_) => {
					if !self.name.source_text().is_case(Case::Pascal) {
						context.add_diagnostic_at(&self.name, DiagnosticInfo::NonPascalCaseGroup {
							original_name: self.name.source_text().to_owned(),
							type_name: value.kind_name().to_owned(),
						});
					}
				},

				_ => {
					if !self.name.source_text().is_case(Case::Snake) {
						context.add_diagnostic_at(&self.name, DiagnosticInfo::NonSnakeCaseName {
							original_name: self.name.source_text().to_owned(),
						});
					}
				},
			}
		}

		context.reassign_variable(&self.name, evaluated.to_owned());

		Declaration {
			name: self.name,
			explicit_type: self.explicit_type.evaluate(context),
			value: evaluated,
			tokens: self.tokens,
			scope: self.scope,
		}
	}
}

impl Spanned for Declaration {
	fn span(&self) -> impl SpanLike {
		self.tokens.keyword_let.span.to(self.value.span())
	}
}
