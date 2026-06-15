use std::fmt::Debug;

use derive_getters::Getters;

use crate::{
	Span,
	Spanned,
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{Expression, block::Block},
		misc::{
			compile_time_parameter::{CompileTimeParameters, EvaluatedCompileTimeParameters},
			parameter::{EvaluatedParameters, Parameters},
			tag::{EvaluatedTagList, TagList},
		},
	},
	comptime::{Evaluate, memory::Pointer},
	diagnostics::Diagnostic,
	if_then_some,
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeRef,
	transpiler::ToCabin,
};

#[derive(Debug, Clone, Getters)]
pub struct Action {
	compile_time_parameters: Option<CompileTimeParameters>,
	parameters: Option<Parameters>,
	return_type: Option<ReturnType>,
	body: Option<Block>,
	parameter_scope: ScopeRef,
	tokens: ActionTokens,
	pub(crate) tags: Option<TagList>,
}

#[derive(Debug, Clone, Getters)]
pub struct ActionTokens {
	keyword_action: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct ReturnType {
	return_type: Expression,
	tokens: ReturnTypeTokens,
}

#[derive(Debug, Clone, Getters)]
pub struct ReturnTypeTokens {
	colon: Token,
}

impl TryParse for Action {
	type Output = Action;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		let keyword_action = tokens.pop(TokenType::KeywordAction, context)?;

		// parameters
		let parameter_scope = context.scope.enter_new_scope(ScopeType::Parameters);
		let compile_time_parameters = CompileTimeParameters::try_parse(tokens, context).ok();
		let parameters = Parameters::try_parse(tokens, context).ok();

		if let Some(compile_time_parameters) = &compile_time_parameters {
			for parameter in compile_time_parameters.parameters() {
				if let Err(error) = context.scope.declare_new_variable_from_id(parameter.name(), context.memory.calloc(), parameter_scope) {
					context.add_diagnostic(Diagnostic {
						span: parameter.name().span(),
						info: error,
						file: context.file.clone(),
					});
				};
			}
		}

		if let Some(parameters) = &parameters {
			for parameter in parameters.parameters() {
				if let Err(error) = context.scope.declare_new_variable_from_id(parameter.name(), context.memory.calloc(), parameter_scope) {
					context.add_diagnostic(Diagnostic {
						span: parameter.name().span(),
						info: error,
						file: context.file.clone(),
					});
				};
			}
		}

		// return Type
		let return_type = if_then_some!(tokens.next_is(TokenType::Colon), {
			let colon = tokens.pop(TokenType::Colon, context)?;
			let expression = Expression::try_parse(tokens, context)?;
			ReturnType {
				return_type: expression,
				tokens: ReturnTypeTokens { colon },
			}
		});

		// body
		let body = Block::try_parse(tokens, context, ScopeType::Action).ok();

		context.scope.exit_scope(parameter_scope).unwrap();

		// Return
		Ok(Action {
			parameters,
			parameter_scope,
			compile_time_parameters,
			return_type,
			body,
			tokens: ActionTokens { keyword_action },
			tags: None,
		})
	}
}

impl Evaluate for Action {
	type Output = EvaluatedAction;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating action");
		let tags = self.tags.evaluate(context);
		let reverter = context.scope.set_current_scope(self.parameter_scope);

		// parameters
		let compile_time_parameters = self.compile_time_parameters.evaluate(context);
		let parameters = self.parameters.evaluate(context);

		// return type
		let return_type = self.return_type.map(|return_type| EvaluatedReturnType {
			return_type: return_type.return_type.evaluate(context),
			tokens: return_type.tokens,
		});

		// body
		if let Some(body) = self.body.as_ref() {
			// body.check(context);
			// must be done in its own pass after evaluation because of self-referential garbage
		}

		reverter.revert(context);

		// Return
		EvaluatedAction {
			compile_time_parameters,
			parameters,
			body: self.body,
			return_type,
			parameter_scope: self.parameter_scope,
			tokens: self.tokens,
			tags,
			this: None,
		}
	}
}

impl Spanned for Action {
	fn span(&self) -> Span {
		let start = self.tokens.keyword_action.span;

		if let Some(body) = &self.body {
			return start.to(body.tokens().right_brace().span);
		}

		if let Some(return_type) = &self.return_type {
			return start.to(return_type.return_type.span());
		}

		if let Some(parameters) = &self.parameters {
			return start.to(parameters.span());
		}

		if let Some(parameters) = &self.compile_time_parameters {
			return start.to(parameters.span());
		}

		start.to(start)
	}
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedAction {
	compile_time_parameters: Option<EvaluatedCompileTimeParameters>,
	parameters: Option<EvaluatedParameters>,
	return_type: Option<EvaluatedReturnType>,
	body: Option<Block>,
	parameter_scope: ScopeRef,
	tokens: ActionTokens,
	tags: Option<EvaluatedTagList>,
	pub(crate) this: Option<Pointer>,
}

#[derive(Debug, Clone, Getters)]
pub struct EvaluatedReturnType {
	return_type: Pointer,
	tokens: ReturnTypeTokens,
}

impl Spanned for EvaluatedAction {
	fn span(&self) -> Span {
		let start = self.tokens.keyword_action.span;

		if let Some(body) = &self.body {
			return start.to(body.tokens().right_brace().span);
		}

		if let Some(return_type) = &self.return_type {
			return start.to(return_type.return_type.span());
		}

		if let Some(parameters) = &self.parameters {
			return start.to(parameters.span());
		}

		if let Some(parameters) = &self.compile_time_parameters {
			return start.to(parameters.span());
		}

		start.to(start)
	}
}

impl ToCabin for Action {
	fn to_cabin(&self) -> String {
		let mut code = "action".to_owned();
		code += &self.compile_time_parameters.to_cabin();
		code += &self.parameters.to_cabin();
		code += &self.return_type.to_cabin();
		code += &self.body.to_cabin();
		code
	}
}

impl EvaluatedAction {
	pub fn takes_this(&self, context: &Context) -> bool {
		self.parameters()
			.as_ref()
			.is_some_and(|parameters| parameters.first().is_some_and(|first| *first.parameter_type() == context.ident("This")))
	}
}

impl ToCabin for ReturnType {
	fn to_cabin(&self) -> String {
		format!(": {}", self.return_type.to_cabin())
	}
}
