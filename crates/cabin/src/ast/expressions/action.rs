use std::fmt::Debug;

use super::parameter::EvaluatedParameter;
use crate::{
	Span,
	Spanned,
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{Expression, block::Block, parameter::Parameter},
		misc::tag::TagList,
	},
	comptime::{CompileTime, memory::ExpressionPointer},
	diagnostics::Diagnostic,
	if_then_else_default,
	if_then_some,
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeId,
	typechecker::Type,
};

#[derive(Debug, Clone)]
pub struct Action {
	tags: TagList,
	compile_time_parameters: Vec<Parameter>,
	parameters: Vec<Parameter>,
	return_type: Option<ExpressionPointer>,
	body: Option<Block>,
	this_object: Option<ExpressionPointer>,
	span: Span,
	parameter_scope_id: ScopeId,
	pub documentation: Option<String>,
}

impl TryParse for Action {
	type Output = Action;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// "action" keyword
		let start = tokens.pop(TokenType::KeywordAction, context)?.span;
		let mut end = start;

		let parameter_scope = context.scope.enter_new_scope(ScopeType::Parameters);

		// Compile-time parameters
		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket), {
			let mut compile_time_parameters = Vec::new();
			end = parse_list!(tokens, context, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens, context)?;
				compile_time_parameters.push(parameter);
			})
			.span;
			compile_time_parameters
		});

		for parameter in &compile_time_parameters {
			if let Err(info) = context.scope.declare_new_variable(&parameter.name, ExpressionPointer::ERROR) {
				context.add_diagnostic(Diagnostic {
					span: parameter.parameter_type.span(context),
					info,
					file: context.file.clone(),
				});
			}
		}

		// Parameters
		let parameters = if_then_else_default!(tokens.next_is(TokenType::LeftParenthesis), {
			let mut parameters = Vec::new();
			end = parse_list!(tokens, context, ListType::Parenthesized, {
				let parameter = Parameter::try_parse(tokens, context)?;
				parameters.push(parameter);
			})
			.span;
			parameters
		});

		// Return Type
		let return_type = if_then_some!(tokens.next_is(TokenType::Colon), {
			let _ = tokens.pop(TokenType::Colon, context)?;
			let expression = Expression::parse(tokens, context);
			end = expression.span(context);
			expression
		});

		// Body
		let body = if_then_some!(tokens.next_is(TokenType::LeftBrace), {
			let block = Block::parse_with_scope_type(tokens, context, ScopeType::Action)?;
			let error = Expression::error(Span::none(), context);
			for parameter in &compile_time_parameters {
				if let Err(error) = context.scope.declare_new_variable_from_id(parameter.name().clone(), error, block.inner_scope_id()) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: parameter.name().span(context),
						info: error,
					});
				};
			}
			for parameter in &parameters {
				if let Err(error) = context.scope.declare_new_variable_from_id(parameter.name().clone(), error, block.inner_scope_id()) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: parameter.name().span(context),
						info: error,
					});
				}
			}
			end = block.span(context);
			block
		});

		context.scope.exit_scope(parameter_scope).unwrap();

		// Return
		Ok(Self {
			tags: TagList::default(),
			parameters,
			parameter_scope_id: parameter_scope,
			compile_time_parameters,
			return_type,
			body,
			this_object: None,
			documentation: None,
			span: start.to(end),
		})
	}
}

impl CompileTime for Action {
	type Output = EvaluatedAction;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		let reverter = context.scope.set_current_scope(self.parameter_scope_id);

		// Compile-time parameters
		let compile_time_parameters = {
			let mut compile_time_parameters = Vec::new();
			for parameter in self.compile_time_parameters {
				compile_time_parameters.push(parameter.evaluate_at_compile_time(context));
			}
			compile_time_parameters
		};

		// Parameters
		let parameters = {
			let mut parameters = Vec::new();
			for parameter in self.parameters {
				parameters.push(parameter.evaluate_at_compile_time(context));
			}
			parameters
		};

		// Return type
		let return_type = self.return_type.map(|return_type| Type::Literal(return_type.evaluate_to_literal(context)));

		// tags
		let tags = self.tags.evaluate_at_compile_time(context);

		// body
		let body = self.body.map(|body| body.evaluate_lazy(context));

		reverter.revert(context);

		// Return
		EvaluatedAction {
			compile_time_parameters,
			parameters,
			body,
			return_type,
			tags,
			span: self.span,
			documentation: self.documentation,
		}
	}
}

impl Spanned for Action {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl Action {
	pub fn set_tags(&mut self, tags: TagList) {
		self.tags = tags;
	}
}

#[derive(Debug, Clone)]
pub struct EvaluatedAction {
	tags: TagList,
	compile_time_parameters: Vec<EvaluatedParameter>,
	parameters: Vec<EvaluatedParameter>,
	return_type: Option<Type>,
	body: Option<Block>,
	span: Span,
	pub documentation: Option<String>,
}

static FUNCTION_DECLARATION_ERROR: EvaluatedAction = EvaluatedAction {
	tags: TagList::empty(),
	compile_time_parameters: Vec::new(),
	parameters: Vec::new(),
	return_type: None,
	body: None,
	span: Span::none(),
	documentation: None,
};

impl EvaluatedAction {
	pub fn compile_time_parameters(&self) -> &[EvaluatedParameter] {
		&self.compile_time_parameters
	}

	pub fn parameters(&self) -> &[EvaluatedParameter] {
		&self.parameters
	}

	pub const fn body(&self) -> Option<&Block> {
		self.body.as_ref()
	}

	pub const fn return_type(&self) -> Option<&Type> {
		self.return_type.as_ref()
	}

	pub const fn tags(&self) -> &TagList {
		&self.tags
	}

	pub fn error() -> &'static EvaluatedAction {
		&FUNCTION_DECLARATION_ERROR
	}
}
