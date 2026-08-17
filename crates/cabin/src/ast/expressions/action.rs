use std::{collections::HashMap, fmt::Debug};

use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{block::Block, object::Object},
		misc::{compile_time_parameter::CompileTimeParameters, parameter::Parameters, tag::TagList},
	},
	comptime::{
		Dot,
		Evaluate,
		storage::Pointer,
		types::{EvaluatedType, Type},
	},
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
	object: Pointer,
	name: Option<String>,
	pub(crate) tags: Option<TagList>,
	pub(crate) this: Option<Pointer>,
}

#[derive(Debug, Clone, Getters)]
pub struct ActionTokens {
	keyword_action: Token,
}

#[derive(Debug, Clone, Getters)]
pub struct ReturnType {
	return_type: Type,
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
				let declaration = parameter.as_declaration(context);
				if let Err(error) = context.scope.declare_new_variable_from_id(parameter.name(), declaration, parameter_scope) {
					context.add_diagnostic(Diagnostic::at(parameter.name(), error));
				};
			}
		}

		if let Some(parameters) = &parameters {
			for parameter in parameters.parameters() {
				let declaration = parameter.as_declaration(context);
				if let Err(error) = context.scope.declare_new_variable_from_id(parameter.name(), declaration, parameter_scope) {
					context.add_diagnostic(Diagnostic::at(parameter.name(), error));
				};
			}
		}

		// return type
		let return_type = if_then_some!(tokens.next_is(TokenType::Colon), {
			let colon = tokens.pop(TokenType::Colon, context)?;
			let inner_return_type = Type::try_parse(tokens, context)?;
			ReturnType {
				return_type: inner_return_type,
				tokens: ReturnTypeTokens { colon },
			}
		});

		// body
		let body = Block::try_parse(tokens, context, ScopeType::Action).ok();

		context.scope.exit_scope(parameter_scope).unwrap();

		let span = || {
			let start = &keyword_action.span;

			if let Some(body) = &body {
				return start.to(body.tokens().right_brace().span());
			}

			if let Some(return_type) = &return_type {
				return start.to(return_type.return_type.span());
			}

			if let Some(parameters) = &parameters {
				return start.to(parameters.span());
			}

			if let Some(compile_time_parameters) = &compile_time_parameters {
				return start.to(compile_time_parameters.span());
			}

			start.to(start)
		};

		// Return
		Ok(Action {
			object: Object::new(Some(EvaluatedType::builtin("Action", context)), HashMap::new(), span()).store(context),
			parameters,
			parameter_scope,
			compile_time_parameters,
			return_type,
			body,
			name: context.get_path(),
			tokens: ActionTokens { keyword_action },
			tags: None,
			this: None,
		})
	}
}

impl Evaluate for Action {
	type Output = Action;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating action");
		let tags = self.tags.evaluate(context);
		let reverter = context.scope.set_current_scope(self.parameter_scope);

		// parameters
		let compile_time_parameters = self.compile_time_parameters.evaluate(context);
		let parameters = self.parameters.evaluate(context);

		// return type
		let return_type = self.return_type.map(|return_type| ReturnType {
			return_type: Type::Evaluated(return_type.return_type.evaluate(context)),
			tokens: return_type.tokens,
		});

		// body
		if let Some(body) = self.body.as_ref() {
			// body.check(context);
			// must be done in its own pass after evaluation because of self-referential garbage
		}

		reverter.revert(context);

		// Return
		Action {
			compile_time_parameters,
			parameters,
			body: self.body,
			name: self.name,
			return_type,
			parameter_scope: self.parameter_scope,
			tokens: self.tokens,
			object: self.object,
			tags,
			this: self.this,
		}
	}
}

impl Spanned for Action {
	fn span(&self) -> impl SpanLike {
		let start = &self.tokens.keyword_action.span;

		if let Some(body) = &self.body {
			return start.to(body.tokens().right_brace().span());
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

impl ToCabin for ReturnType {
	fn to_cabin(&self) -> String {
		format!(": {}", self.return_type.to_cabin())
	}
}

impl Action {
	pub fn takes_this(&self) -> bool {
		self.parameters
			.as_ref()
			.is_some_and(|parameters| parameters.first().is_some_and(|parameter| parameter.name().source_text() == "this"))
	}

	pub fn bind(mut self, object: Object, context: &mut Context) -> Action {
		self.this = Some(object.store(context));
		self
	}
}

impl Dot for Action {
	fn dot(&self, context: &mut Context, name: &str) -> Option<Pointer> {
		self.object.dot(context, name)
	}
}
