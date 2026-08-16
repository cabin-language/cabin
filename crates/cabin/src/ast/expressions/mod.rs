use std::fmt::Debug;

// This is required because of a bug in `try_as`
use try_as::{
	macros::{From, TryAsMut, TryAsRef},
	traits::{self as try_as_traits, TryAsMut},
};

use crate::{
	Span,
	SpanLike,
	Spanned,
	api::context::Context,
	ast::{
		expressions::{
			action::Action,
			binary::BinaryExpression,
			block::Block,
			either::Either,
			error::ErrorExpression,
			extend::Extend,
			field_access::FieldAccess,
			foreach::ForLoop,
			function_call::FunctionCall,
			group::Group,
			identifier::Identifier,
			if_expression::IfExpression,
			list::List,
			new::NewExpression,
			number::Number,
			parenthesized::ParenthesizedExpression,
			run::{RunExpression, Runtime},
			string::Text,
			unary::UnaryExpression,
		},
		misc::tag::TagList,
	},
	comptime::{Evaluate, storage::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	transpiler::ToCabin,
};

pub mod action;
pub mod binary;
pub mod block;
pub mod either;
pub mod error;
pub mod extend;
pub mod field_access;
pub mod foreach;
pub mod function_call;
pub mod group;
pub mod identifier;
pub mod if_expression;
pub mod list;
pub mod new;
pub mod number;
pub mod object;
pub mod parenthesized;
pub mod run;
pub mod string;
pub mod unary;
pub mod while_loop;

#[derive(Debug, Clone, From, try_as::macros::TryInto, TryAsRef, TryAsMut)]
pub enum Expression {
	Number(Number),
	Text(Text),
	Identifier(Identifier),
	List(List),

	Action(Action),
	Group(Group),
	Either(Either),
	New(NewExpression),
	Extend(Extend),

	Block(Block),
	FieldAccess(FieldAccess),
	FunctionCall(FunctionCall),
	If(IfExpression),
	Binary(BinaryExpression),
	ForEachLoop(ForLoop),
	Run(RunExpression),
	Unary(UnaryExpression),

	Parenthesized(ParenthesizedExpression),
	ErrorExpression(ErrorExpression),
}

impl TryParse for Expression {
	type Output = Pointer;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		log!(context, "Parsing expression");
		let result = BinaryExpression::try_parse(tokens, context);
		match result {
			Ok(expression) => Ok(expression),
			Err(error) => {
				log!(context, "Error: Parsing expression: {}", error.info());
				context.add_diagnostic(error);
				let mut error_tokens = Vec::new();
				if let Ok(token_type) = tokens.peek_type(context) {
					error_tokens.push(tokens.pop(token_type, context).unwrap());
					Ok(Expression::ErrorExpression(ErrorExpression::new(error_tokens)).store(context))
				} else {
					Err(Diagnostic::at(&Span::eof(&context.file), DiagnosticInfo::UnexpectedGenericEOF))
				}
			},
		}
	}
}

impl Evaluate for Expression {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating expression");
		match self {
			Self::Block(block) => block.evaluate(context),
			Self::Action(action) => Expression::from(action.evaluate(context)).store(context),
			Self::Group(group) => Expression::from(group.evaluate(context)).store(context),
			Self::Either(either) => Expression::from(either.evaluate(context)).store(context),
			Self::Number(number) => Expression::from(number).store(context),
			Self::Text(text) => text.evaluate(context),
			Self::FieldAccess(field_access) => field_access.evaluate(context),
			Self::FunctionCall(function_call) => function_call.evaluate(context),
			Self::If(if_expression) => if_expression.evaluate(context),
			Self::Identifier(name) => name.evaluate(context),
			Self::New(new) => Expression::from(new.evaluate(context)).store(context),
			Self::ForEachLoop(for_loop) => for_loop.evaluate(context),
			Self::Extend(extend) => Expression::from(extend.evaluate(context)).store(context),
			Self::Run(run_expression) => Expression::from(run_expression.evaluate(context)).store(context),
			Self::Binary(binary_expression) => binary_expression.evaluate(context),
			Self::Parenthesized(parenthesized_expression) => Expression::from(parenthesized_expression.evaluate(context)).store(context),
			Self::ErrorExpression(error) => Expression::from(error).store(context),
			Self::List(list) => Expression::from(list.evaluate(context)).store(context),
			Self::Unary(_) => todo!(),
		}
	}
}

impl Runtime for Expression {
	fn evaluate_subexpressions(self, context: &mut Context) -> Self {
		match self {
			Self::FunctionCall(function_call) => Expression::FunctionCall(function_call.evaluate_subexpressions(context)),
			Self::Binary(binary) => Expression::Binary(binary.evaluate_subexpressions(context)),
			_ => {
				context.add_diagnostic(Diagnostic::at(&self, DiagnosticInfo::RunLiteral));
				self
			},
		}
	}
}

impl Expression {
	pub fn store(self, context: &mut Context) -> Pointer {
		context.storage.store(self)
	}

	pub fn set_tags(&mut self, tags: TagList) {
		match self {
			Self::Action(action) => action.tags = Some(tags),
			_ => {},
		}
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		match self {
			Expression::Number(_) => true,
			Expression::Group(group) => group.is_fully_known(context),
			Expression::Text(text) => text.is_fully_known(context),
			_ => false,
		}
	}

	pub const fn kind_name(&self) -> &'static str {
		match self {
			Expression::Block(_) => "block",
			Expression::Action(_) => "action",
			Expression::Extend(_) => "extension",
			Expression::FieldAccess(_) => "field access",
			Expression::If(_) => "if expression",
			Expression::Identifier(_) => "name",
			Expression::New(_) => "object constructor",
			Expression::Run(_) => "run expression",
			Expression::List(_) => "list",
			Expression::Group(_) => "group",
			Expression::Either(_) => "either",
			Expression::FunctionCall(_) => "action call",
			Expression::Number(_) => "number",
			Expression::ForEachLoop(_) => "for loop",
			Expression::ErrorExpression(_) => "error",
			Expression::Parenthesized(_) => "parenthesized expression",
			Expression::Text(_) => "text",
			Expression::Binary(_) => "binary expression",
			Expression::Unary(_) => "unary expression",
		}
	}
}

impl ToCabin for Expression {
	fn to_cabin(&self) -> String {
		match self {
			Expression::Action(action) => action.to_cabin(),
			Expression::Block(block) => block.to_cabin(),
			Expression::Identifier(identifier) => identifier.to_cabin(),
			Expression::Run(run_expression) => run_expression.to_cabin(),
			Expression::List(list) => list.to_cabin(),
			Expression::Number(number) => number.to_cabin(),
			Expression::Text(text) => text.to_cabin(),
			Expression::Binary(binary_expression) => binary_expression.to_cabin(),
			Expression::Parenthesized(parenthesized_expression) => parenthesized_expression.to_cabin(),
			Expression::Either(either) => either.to_cabin(),
			Expression::ForEachLoop(for_loop) => for_loop.to_cabin(),
			Expression::New(new_expression) => new_expression.to_cabin(),

			Expression::FieldAccess(field_access) => todo!(),
			Expression::FunctionCall(function_call) => todo!(),
			Expression::Group(group) => todo!(),
			Expression::If(if_expression) => todo!(),
			Expression::Unary(unary_expression) => todo!(),
			Expression::Extend(extend) => todo!(),

			Expression::ErrorExpression(error_expression) => todo!(),
		}
	}
}

impl Spanned for Expression {
	fn span(&self) -> impl SpanLike {
		match self {
			Expression::Identifier(name) => name.span().owned(),
			Expression::Run(run_expression) => run_expression.span().owned(),
			Expression::Block(block) => block.span().owned(),
			Expression::New(object_constructor) => object_constructor.span().owned(),
			Expression::FunctionCall(function_call) => function_call.span().owned(),
			Expression::If(if_expression) => if_expression.span().owned(),
			Expression::FieldAccess(field_access) => field_access.span().owned(),
			Expression::ForEachLoop(for_each_loop) => for_each_loop.span().owned(),
			Expression::Unary(unary) => unary.span().owned(),
			Expression::List(list) => list.span().owned(),
			Expression::Group(group) => group.span().owned(),
			Expression::Either(either) => either.span().owned(),
			Expression::Binary(binary_expression) => binary_expression.span().owned(),
			Expression::Number(number) => number.span().owned(),
			Expression::Text(text) => text.span().owned(),
			Expression::Action(action) => action.span().owned(),
			Expression::Extend(extend) => extend.span().owned(),
			Expression::Parenthesized(parenthesized_expression) => parenthesized_expression.span().owned(),
			Expression::ErrorExpression(error_expression) => error_expression.span().owned(),
		}
	}
}
