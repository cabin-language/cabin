use std::fmt::Debug;

use super::parameter::EvaluatedParameter;
use crate::{
	api::{context::Context, scope::ScopeType},
	ast::{
		expressions::{block::Block, parameter::Parameter, Expression},
		misc::tag::TagList,
	},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	if_then_else_default,
	if_then_some,
	lexer::TokenType,
	parse_list,
	parser::{ListType, Parse as _, TokenQueue, TokenQueueFunctionality as _, TryParse},
	Span,
	Spanned,
};

#[derive(Debug, Clone)]
pub struct FunctionDeclaration {
	tags: TagList,
	compile_time_parameters: Vec<Parameter>,
	parameters: Vec<Parameter>,
	return_type: Option<ExpressionPointer>,
	body: Option<Block>,
	this_object: Option<ExpressionPointer>,
	span: Span,
}

impl TryParse for FunctionDeclaration {
	type Output = FunctionDeclaration;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		// "function" keyword
		let start = tokens.pop(TokenType::KeywordAction, context)?.span;
		let mut end = start;

		// Compile-time parameters
		let compile_time_parameters = if_then_else_default!(tokens.next_is(TokenType::LeftAngleBracket, context), {
			let mut compile_time_parameters = Vec::new();
			end = parse_list!(tokens, context, ListType::AngleBracketed, {
				let parameter = Parameter::try_parse(tokens, context)?;
				compile_time_parameters.push(parameter);
			})
			.span;
			compile_time_parameters
		});

		// Parameters
		let parameters = if_then_else_default!(tokens.next_is(TokenType::LeftParenthesis, context), {
			let mut parameters = Vec::new();
			end = parse_list!(tokens, context, ListType::Parenthesized, {
				let parameter = Parameter::try_parse(tokens, context)?;
				parameters.push(parameter);
			})
			.span;
			parameters
		});

		// Return Type
		let return_type = if_then_some!(tokens.next_is(TokenType::Colon, context), {
			let _ = tokens.pop(TokenType::Colon, context)?;
			let expression = Expression::parse(tokens, context);
			end = expression.span(context);
			expression
		});

		// Body
		let body = if_then_some!(tokens.next_is(TokenType::LeftBrace, context), {
			let block = Block::parse_with_scope_type(tokens, context, ScopeType::Function)?;
			let error = Expression::error(Span::unknown(), context);
			for parameter in &compile_time_parameters {
				if let Err(error) = context.scope_tree.declare_new_variable_from_id(parameter.name().clone(), error, block.inner_scope_id()) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: parameter.name().span(context),
						info: DiagnosticInfo::Error(error),
					});
				};
			}
			for parameter in &parameters {
				let error = Expression::error(Span::unknown(), context);
				if let Err(error) = context.scope_tree.declare_new_variable_from_id(parameter.name().clone(), error, block.inner_scope_id()) {
					context.add_diagnostic(Diagnostic {
						file: context.file.clone(),
						span: parameter.name().span(context),
						info: DiagnosticInfo::Error(error),
					});
				}
			}
			end = block.span(context);
			block
		});

		// Return
		Ok(Self {
			tags: TagList::default(),
			parameters,
			compile_time_parameters,
			return_type,
			body,
			this_object: None,
			span: start.to(end),
		})
	}
}

impl CompileTime for FunctionDeclaration {
	type Output = EvaluatedFunctionDeclaration;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
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
		let return_type = self.return_type.map(|return_type| return_type.evaluate_to_literal(context));

		let tags = self.tags.evaluate_at_compile_time(context);

		let body = self.body.map(|body| body.evaluate_at_compile_time(context));

		// Return
		let function = EvaluatedFunctionDeclaration {
			compile_time_parameters,
			parameters,
			body,
			return_type,
			tags,
			span: self.span,
		};

		function
	}
}

impl Spanned for FunctionDeclaration {
	fn span(&self, _context: &Context) -> Span {
		self.span
	}
}

impl FunctionDeclaration {
	pub(crate) fn set_tags(&mut self, tags: TagList) {
		self.tags = tags;
	}
}

#[derive(Debug, Clone)]
pub struct EvaluatedFunctionDeclaration {
	tags: TagList,
	compile_time_parameters: Vec<EvaluatedParameter>,
	parameters: Vec<EvaluatedParameter>,
	return_type: Option<LiteralPointer>,
	body: Option<Block>,
	span: Span,
}

static FUNCTION_DECLARATION_ERROR: EvaluatedFunctionDeclaration = EvaluatedFunctionDeclaration {
	tags: TagList::empty(),
	compile_time_parameters: Vec::new(),
	parameters: Vec::new(),
	return_type: None,
	body: None,
	span: Span::unknown(),
};

impl EvaluatedFunctionDeclaration {
	pub(crate) fn compile_time_parameters(&self) -> &[EvaluatedParameter] {
		&self.compile_time_parameters
	}

	pub(crate) fn parameters(&self) -> &[EvaluatedParameter] {
		&self.parameters
	}

	pub(crate) fn body(&self) -> Option<&Block> {
		self.body.as_ref()
	}

	pub(crate) fn tags(&self) -> &TagList {
		&self.tags
	}

	pub(crate) fn error() -> &'static EvaluatedFunctionDeclaration {
		&FUNCTION_DECLARATION_ERROR
	}
}
