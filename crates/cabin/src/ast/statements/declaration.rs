use convert_case::{Case, Casing as _};
use derive_getters::Getters;

use crate::{
	Span,
	Spanned,
	api::context::Context,
	ast::{
		expressions::{Expression, evaluated::EvaluatedExpression, identifier::Identifier},
		misc::tag::TagList,
		statements::Statement,
	},
	comptime::{
		Evaluate,
		memory::{Pointer, Storable},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::{Token, TokenType},
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeRef,
};

#[derive(Debug, Clone, Getters)]
pub struct Declaration {
	name: Identifier,
	explicit_type: Option<DeclarationType>,
	value: Expression,
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
}

#[derive(Debug, Clone, Getters)]
pub struct DeclarationType {
	value: Expression,
	tokens: DeclarationTypeTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct DeclarationTypeTokens {
	colon: Token,
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
		let keyword_editable = if_then_some!(tokens.next_is(TokenType::KeywordEditable), tokens.pop(TokenType::KeywordVisible, context).unwrap());

		// name
		let name = Identifier::try_parse(tokens, context)?;

		let explicit_type = if_then_some!(tokens.next_is(TokenType::Colon), {
			let colon = tokens.pop(TokenType::Colon, context).unwrap();
			DeclarationType {
				value: Expression::try_parse(tokens, context)?,
				tokens: DeclarationTypeTokens { colon },
			}
		});

		// Value
		let equal = tokens.pop(TokenType::Equal, context)?;

		let value = Expression::try_parse(tokens, context)?;

		// Add the name declaration to the scope
		if let Err(error) = context.scope.declare_new_variable(name.clone(), context.memory.calloc()) {
			context.add_diagnostic(Diagnostic {
				file: context.file.clone(),
				span: name.span(),
				info: error,
			});
		}

		let semicolon = tokens.pop(TokenType::Semicolon, context)?;

		// Return the declaration
		Ok(Statement::Declaration(Declaration {
			name,
			value,
			explicit_type,
			scope: context.scope.unique_id(),
			tokens: DeclarationTokens {
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
	type Output = EvaluatedDeclaration;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		let evaluated = self.value.evaluate(context);

		if let Storable::Expression(value) = evaluated.value(context) {
			match value {
				EvaluatedExpression::Group(_) | EvaluatedExpression::Either(_) => {
					if !self.name.source_identifier().is_case(Case::Pascal) {
						context.add_diagnostic(Diagnostic {
							file: context.file.clone(),
							span: self.name.span(),
							info: DiagnosticInfo::NonPascalCaseGroup {
								original_name: self.name.source_identifier().to_owned(),
								type_name: value.kind_name().to_owned(),
							},
						});
					}
				},

				_ => {
					if !self.name.source_identifier().is_case(Case::Snake) {
						context.add_diagnostic(Diagnostic {
							file: context.file.clone(),
							span: self.name.span(),
							info: DiagnosticInfo::NonSnakeCaseName {
								original_name: self.name.source_identifier().to_owned(),
							},
						});
					}
				},
			}
		}

		context.scope.reassign_variable_from_id(&self.name, evaluated, self.scope);

		EvaluatedDeclaration {
			name: self.name,
			explicit_type: self.explicit_type.map(|t| EvaluatedDeclarationType {
				value: t.value.evaluate(context),
				tokens: t.tokens,
			}),
			value: evaluated,
			tokens: self.tokens,
		}
	}
}

impl Spanned for Declaration {
	fn span(&self) -> Span {
		self.tokens.keyword_let.span.to(self.value.span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedDeclaration {
	name: Identifier,
	explicit_type: Option<EvaluatedDeclarationType>,
	value: Pointer,
	tokens: DeclarationTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedDeclarationType {
	value: Pointer,
	tokens: DeclarationTypeTokens,
}

impl Spanned for EvaluatedDeclaration {
	fn span(&self) -> Span {
		self.tokens.keyword_let.span.to(self.value.span())
	}
}
