use derive_getters::Getters;

use crate::{
	Context,
	SpanLike,
	Spanned,
	api::scope::ScopeType,
	ast::{
		expressions::{UnevaluatedExpression, identifier::Identifier},
		misc::{
			compile_time_parameter::{CompileTimeParameters, EvaluatedCompileTimeParameters},
			tag::{EvaluatedTagList, TagList},
		},
	},
	comptime::{
		Evaluate,
		storage::{Evaluated, Pointer},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_some,
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	util::ExitScope as _,
};

#[derive(Debug, Clone, Getters)]
pub struct Extend {
	compile_time_parameters: Option<CompileTimeParameters>,
	type_to_extend: UnevaluatedExpression,
	as_clause: Option<ExtendAsClause>,
	fields: Vec<ExtendField>,
	tokens: ExtendTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ExtendField {
	tags: Option<TagList>,
	name: Identifier,
	value: UnevaluatedExpression,
	tokens: ExtendFieldTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ExtendFieldTokens {
	equal: Token,
	comma: Option<Token>,
}

#[derive(Debug, Clone, Getters)]
pub struct ExtendAsClause {
	as_type: UnevaluatedExpression,
	tokens: ExtendAsClauseTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ExtendAsClauseTokens {
	keyword_as: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct ExtendTokens {
	keyword_extend: Token,
	left_brace: Token,
	right_brace: Token,
}

impl TryParse for Extend {
	type Output = Extend;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_extend = tokens.pop(TokenType::KeywordExtend, context)?;

		let extend_scope = context.scope.enter_new_scope(ScopeType::Extend);

		let compile_time_parameters = CompileTimeParameters::try_parse(tokens, context).ok();
		if let Some(compile_time_parameters) = &compile_time_parameters {
			for parameter in compile_time_parameters.parameters() {
				if let Err(error) = context
					.scope
					.declare_new_variable_from_id(parameter.name(), parameter.as_declaration(context), extend_scope)
				{
					context.add_diagnostic(Diagnostic::at(parameter.name(), error));
				};
			}
		}

		let type_to_extend = UnevaluatedExpression::try_parse(tokens, context).or_exit_scope(context)?;

		let as_clause = if_then_some!(tokens.next_is(TokenType::KeywordAs), {
			(
				tokens.pop(TokenType::KeywordAs, context).or_exit_scope(context)?,
				UnevaluatedExpression::try_parse(tokens, context).or_exit_scope(context)?,
			)
		});

		let mut fields = Vec::new();
		let left_brace = tokens.pop(TokenType::LeftBrace, context).or_exit_scope(context)?;
		while !tokens.next_is(TokenType::RightBrace) {
			// tags
			let tags = if_then_some!(tokens.next_is(TokenType::TagOpening), TagList::try_parse(tokens, context).or_exit_scope(context)?);

			// name
			let name = Identifier::try_parse(tokens, context).or_exit_scope(context)?;

			// value
			let equal = tokens.pop(TokenType::Equal, context).or_exit_scope(context)?;
			let mut value = UnevaluatedExpression::try_parse(tokens, context).or_exit_scope(context)?;
			if let Some(tags) = &tags {
				value.set_tags(tags.clone());
			}

			let comma = tokens.pop(TokenType::Comma, context).ok();
			let has_comma = comma.is_some();

			// add field
			fields.push(ExtendField {
				tags,
				name,
				value,
				tokens: ExtendFieldTokens { equal, comma },
			});

			if !has_comma {
				break;
			}
		}

		let right_brace = tokens.pop(TokenType::RightBrace, context).or_exit_scope(context)?;

		let extend = Extend {
			type_to_extend,
			as_clause: as_clause.map(|(keyword_as, as_type)| ExtendAsClause {
				as_type,
				tokens: ExtendAsClauseTokens { keyword_as },
			}),
			fields,
			compile_time_parameters,
			tokens: ExtendTokens {
				keyword_extend,
				left_brace,
				right_brace,
			},
		};

		if extend.fields.is_empty() && extend.as_clause.is_none() {
			context.add_diagnostic(Diagnostic::at(&extend, DiagnosticInfo::EmptyExtension));
		}

		context.scope.exit_scope(extend_scope).unwrap();

		Ok(extend)
	}
}

impl Evaluate for Extend {
	type Output = EvaluatedExtend;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating expression");
		let type_to_extend = self.type_to_extend.evaluate(context);
		let as_clause = self.as_clause.map(|as_clause| EvaluatedExtendAsClause {
			as_type: as_clause.as_type.evaluate(context),
			tokens: as_clause.tokens,
		});

		let mut fields = Vec::new();
		for field in self.fields {
			fields.push(EvaluatedExtendField {
				tags: field.tags.evaluate(context),
				name: field.name,
				value: field.value.evaluate(context),
				tokens: field.tokens,
			});
		}

		// Evaluate compile-time parameters
		let compile_time_parameters = self.compile_time_parameters.evaluate(context);

		EvaluatedExtend {
			type_to_extend,
			as_clause,
			fields,
			compile_time_parameters,
			tokens: self.tokens,
		}
	}
}

impl Spanned for Extend {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_extend().span().to(self.tokens().right_brace().span())
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedExtend {
	compile_time_parameters: Option<EvaluatedCompileTimeParameters>,
	type_to_extend: Pointer<Evaluated>,
	as_clause: Option<EvaluatedExtendAsClause>,
	fields: Vec<EvaluatedExtendField>,
	tokens: ExtendTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedExtendField {
	tags: Option<EvaluatedTagList>,
	name: Identifier,
	value: Pointer<Evaluated>,
	tokens: ExtendFieldTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedExtendAsClause {
	as_type: Pointer<Evaluated>,
	tokens: ExtendAsClauseTokens,
}

impl Spanned for EvaluatedExtend {
	fn span(&self) -> impl SpanLike {
		self.tokens().keyword_extend().span().to(self.tokens().right_brace().span())
	}
}
