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
			evaluated::EvaluatedExpression,
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
			run::{RunExpression, Runtimeable},
			string::Text,
			unary::UnaryExpression,
		},
		misc::tag::TagList,
	},
	comptime::{
		Evaluate,
		storage::{EvaluatedRef, ExpressionRef, Storable, StorageRef},
	},
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
pub mod evaluated;
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
pub enum UnevaluatedExpression {
	Number(Number),
	Text(Text),
	Identifier(Identifier),
	List(List),

	Action(Box<Action>),
	Group(Group),
	Either(Either),
	New(NewExpression),
	Extend(Box<Extend>),

	Block(Block),
	FieldAccess(Box<FieldAccess>),
	FunctionCall(Box<FunctionCall>),
	If(Box<IfExpression>),
	Binary(Box<BinaryExpression>),
	ForEachLoop(Box<ForLoop>),
	Run(Box<RunExpression>),
	Unary(Box<UnaryExpression>),

	Parenthesized(Box<ParenthesizedExpression>),
	ErrorExpression(ErrorExpression),
}

impl TryParse for UnevaluatedExpression {
	type Output = ExpressionRef;

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
					Ok(UnevaluatedExpression::ErrorExpression(ErrorExpression::new(error_tokens)).store(context))
				} else {
					Err(Diagnostic::at(&Span::eof(&context.file), DiagnosticInfo::UnexpectedGenericEOF))
				}
			},
		}
	}
}

impl Evaluate for UnevaluatedExpression {
	type Output = EvaluatedRef;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating expression");
		match self {
			Self::Block(block) => block.evaluate(context),
			Self::Action(action) => EvaluatedExpression::Action(action.evaluate(context)).store(context),
			Self::Group(group) => EvaluatedExpression::Group(group.evaluate(context)).store(context),
			Self::Either(either) => EvaluatedExpression::Either(either.evaluate(context)).store(context),
			Self::Number(number) => EvaluatedExpression::Number(number).store(context),
			Self::Text(text) => text.evaluate(context),
			Self::FieldAccess(field_access) => field_access.evaluate(context),
			Self::FunctionCall(function_call) => function_call.evaluate(context),
			Self::If(if_expression) => if_expression.evaluate(context),
			Self::Identifier(name) => name.evaluate(context),
			Self::New(new) => new.evaluate(context),
			Self::ForEachLoop(for_loop) => for_loop.evaluate(context),
			Self::Extend(extend) => EvaluatedExpression::Extend(extend.evaluate(context)).store(context),
			Self::Run(run_expression) => EvaluatedExpression::Run(Box::new(run_expression.evaluate(context))).store(context),
			Self::Binary(binary_expression) => binary_expression.evaluate(context),
			Self::Parenthesized(parenthesized_expression) => EvaluatedExpression::Parenthesized(parenthesized_expression.evaluate(context)).store(context),
			Self::Unary(unary_expression) => unary_expression.evaluate(context),
			Self::ErrorExpression(error) => EvaluatedExpression::ErrorExpression(error).store(context),
			Self::List(list) => EvaluatedExpression::List(list.evaluate(context)).store(context),
		}
	}
}

impl Runtimeable for UnevaluatedExpression {
	type Output = EvaluatedExpression;

	fn evaluate_subexpressions(self, context: &mut Context) -> EvaluatedExpression {
		match self {
			Self::FunctionCall(function_call) => EvaluatedExpression::FunctionCall(function_call.evaluate_subexpressions(context)),
			_ => {
				context.add_diagnostic(Diagnostic::at(&self, DiagnosticInfo::RunLiteral));

				// erm.....
				self.evaluate(context).value(context).as_expression().unwrap().clone()
			},
		}
	}
}

impl UnevaluatedExpression {
	pub fn store(self, context: &mut Context) -> ExpressionRef {
		context.storage.store(self)
	}

	pub fn set_tags(&mut self, tags: TagList) {
		match self {
			Self::Action(action) => action.tags = Some(tags),
			_ => {},
		}
	}

	pub const fn kind_name(&self) -> &'static str {
		match self {
			UnevaluatedExpression::Block(_) => "block",
			UnevaluatedExpression::Action(_) => "action",
			UnevaluatedExpression::Extend(_) => "extension",
			UnevaluatedExpression::FieldAccess(_) => "field access",
			UnevaluatedExpression::If(_) => "if expression",
			UnevaluatedExpression::Identifier(_) => "name",
			UnevaluatedExpression::New(_) => "object constructor",
			UnevaluatedExpression::Run(_) => "run expression",
			UnevaluatedExpression::List(_) => "list",
			UnevaluatedExpression::Group(_) => "group",
			UnevaluatedExpression::Either(_) => "either",
			UnevaluatedExpression::FunctionCall(_) => "action call",
			UnevaluatedExpression::Number(_) => "number",
			UnevaluatedExpression::ForEachLoop(_) => "for loop",
			UnevaluatedExpression::ErrorExpression(_) => "error",
			UnevaluatedExpression::Parenthesized(_) => "parenthesized expression",
			UnevaluatedExpression::Text(_) => "text",
			UnevaluatedExpression::Binary(_) => "binary expression",
			UnevaluatedExpression::Unary(_) => "unary expression",
		}
	}
}

impl ToCabin for UnevaluatedExpression {
	fn to_cabin(&self) -> String {
		match self {
			UnevaluatedExpression::Action(action) => action.to_cabin(),
			UnevaluatedExpression::Block(block) => block.to_cabin(),
			UnevaluatedExpression::Identifier(identifier) => identifier.to_cabin(),
			UnevaluatedExpression::Run(run_expression) => run_expression.to_cabin(),
			UnevaluatedExpression::List(list) => list.to_cabin(),
			UnevaluatedExpression::Number(number) => number.to_cabin(),
			UnevaluatedExpression::Text(text) => text.to_cabin(),
			UnevaluatedExpression::Binary(binary_expression) => binary_expression.to_cabin(),
			UnevaluatedExpression::Parenthesized(parenthesized_expression) => parenthesized_expression.to_cabin(),
			UnevaluatedExpression::Either(either) => either.to_cabin(),
			UnevaluatedExpression::ForEachLoop(for_loop) => for_loop.to_cabin(),
			UnevaluatedExpression::New(new_expression) => new_expression.to_cabin(),

			UnevaluatedExpression::FieldAccess(field_access) => todo!(),
			UnevaluatedExpression::FunctionCall(function_call) => todo!(),
			UnevaluatedExpression::Group(group) => todo!(),
			UnevaluatedExpression::If(if_expression) => todo!(),
			UnevaluatedExpression::Unary(unary_expression) => todo!(),
			UnevaluatedExpression::Extend(extend) => todo!(),

			UnevaluatedExpression::ErrorExpression(error_expression) => todo!(),
		}
	}
}

impl Spanned for UnevaluatedExpression {
	fn span(&self) -> impl SpanLike {
		match self {
			UnevaluatedExpression::Identifier(name) => name.span().owned(),
			UnevaluatedExpression::Run(run_expression) => run_expression.span().owned(),
			UnevaluatedExpression::Block(block) => block.span().owned(),
			UnevaluatedExpression::New(object_constructor) => object_constructor.span().owned(),
			UnevaluatedExpression::FunctionCall(function_call) => function_call.span().owned(),
			UnevaluatedExpression::If(if_expression) => if_expression.span().owned(),
			UnevaluatedExpression::FieldAccess(field_access) => field_access.span().owned(),
			UnevaluatedExpression::ForEachLoop(for_each_loop) => for_each_loop.span().owned(),
			UnevaluatedExpression::Unary(unary) => unary.span().owned(),
			UnevaluatedExpression::List(list) => list.span().owned(),
			UnevaluatedExpression::Group(group) => group.span().owned(),
			UnevaluatedExpression::Either(either) => either.span().owned(),
			UnevaluatedExpression::Binary(binary_expression) => binary_expression.span().owned(),
			UnevaluatedExpression::Number(number) => number.span().owned(),
			UnevaluatedExpression::Text(text) => text.span().owned(),
			UnevaluatedExpression::Action(action) => action.span().owned(),
			UnevaluatedExpression::Extend(extend) => extend.span().owned(),
			UnevaluatedExpression::Parenthesized(parenthesized_expression) => parenthesized_expression.span().owned(),
			UnevaluatedExpression::ErrorExpression(error_expression) => error_expression.span().owned(),
		}
	}
}

#[derive(Debug, Clone, TryAsRef, TryAsMut, From)]
pub enum Expression {
	Unevaluated(UnevaluatedExpression),
	Evaluated(EvaluatedExpression),
}

impl Spanned for Expression {
	fn span(&self) -> impl SpanLike {
		match self {
			Expression::Unevaluated(unevaluated_expression) => unevaluated_expression.span().owned(),
			Expression::Evaluated(evaluated_expression) => evaluated_expression.span().owned(),
		}
	}
}

impl TryParse for Expression {
	type Output = ExpressionRef;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		UnevaluatedExpression::try_parse(tokens, context).map(|expr| Expression::Unevaluated(expr).store(context))
	}
}

impl Expression {
	pub fn is_fully_known(&self) -> bool {
		self.as_evaluated().is_some_and(|expression| expression.is_fully_known())
	}

	fn store(self, context: &mut Context) -> ExpressionRef {
		context.storage.store(self)
	}

	pub fn as_evaluated(&self) -> Option<&EvaluatedExpression> {
		match self {
			Expression::Evaluated(evaluated) => Some(evaluated),
			Expression::Unevaluated(_) => None,
		}
	}

	pub fn as_unevaluated(&self) -> Option<&UnevaluatedExpression> {
		match self {
			Expression::Unevaluated(expression) => Some(expression),
			Expression::Evaluated(_) => None,
		}
	}

	pub fn as_unevaluated_mut(&mut self) -> Option<&mut UnevaluatedExpression> {
		match self {
			Expression::Unevaluated(expression) => Some(expression),
			Expression::Evaluated(_) => None,
		}
	}

	pub fn as_evaluated_mut(&mut self) -> Option<&mut EvaluatedExpression> {
		match self {
			Expression::Evaluated(evaluated) => Some(evaluated),
			Expression::Unevaluated(_) => None,
		}
	}
}
