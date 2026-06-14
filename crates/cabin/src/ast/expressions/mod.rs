use std::fmt::Debug;

// This is required because of a bug in `try_as`
use try_as::{
	macros::{From, TryAsMut, TryAsRef},
	traits::{self as try_as_traits, TryAsMut},
};

use crate::{
	Span,
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
	comptime::{Evaluate, memory::Pointer},
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
pub enum Expression {
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

impl TryParse for Expression {
	type Output = Expression;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		log!(context, "Parsing expression");
		let result = BinaryExpression::try_parse(tokens, context);
		match result {
			Ok(expression) => Ok(expression),
			Err(error) => {
				log!(context, "Error: Parsing expression: {}", error.info);
				context.add_diagnostic(error);
				let mut error_tokens = Vec::new();
				if let Ok(token_type) = tokens.peek_type(context) {
					error_tokens.push(tokens.pop(token_type, context).unwrap());
					Ok(Expression::ErrorExpression(ErrorExpression::new(error_tokens)))
				} else {
					Err(Diagnostic {
						span: Span::none(),
						info: DiagnosticInfo::UnexpectedGenericEOF,
						file: context.file.clone(),
					})
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

impl Runtimeable for Expression {
	type Output = EvaluatedExpression;

	fn evaluate_subexpressions(self, context: &mut Context) -> EvaluatedExpression {
		match self {
			Self::FunctionCall(function_call) => EvaluatedExpression::FunctionCall(function_call.evaluate_subexpressions(context)),
			_ => {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					span: self.span(),
					info: DiagnosticInfo::RunLiteral,
				});

				// erm.....
				self.evaluate(context).value(context).as_expression().unwrap().clone()
			},
		}
	}
}

impl Expression {
	pub fn set_tags(&mut self, tags: TagList) {
		match self {
			Self::Action(action) => action.tags = Some(tags),
			_ => {},
		}
	}
}

impl ToCabin for Expression {
	fn to_cabin(&self) -> String {
		match self {
			Expression::Block(block) => todo!(),
			Expression::FieldAccess(field_access) => todo!(),
			Expression::FunctionCall(function_call) => todo!(),
			Expression::Group(group) => todo!(),
			Expression::Either(either) => todo!(),
			Expression::If(if_expression) => todo!(),
			Expression::Binary(binary_expression) => todo!(),
			Expression::Identifier(identifier) => todo!(),
			Expression::New(new_expression) => todo!(),
			Expression::Number(number) => todo!(),
			Expression::Text(text) => todo!(),
			Expression::ForEachLoop(for_loop) => todo!(),
			Expression::Run(run_expression) => todo!(),
			Expression::Unary(unary_expression) => todo!(),
			Expression::Parenthesized(parenthesized_expression) => todo!(),
			Expression::List(list) => todo!(),
			Expression::ErrorExpression(error_expression) => todo!(),
			Expression::Action(action) => todo!(),
			Expression::Extend(extend) => todo!(),
		}
	}
}

impl Spanned for Expression {
	fn span(&self) -> Span {
		match self {
			Expression::Identifier(name) => name.span(),
			Expression::Run(run_expression) => run_expression.span(),
			Expression::Block(block) => block.span(),
			Expression::New(object_constructor) => object_constructor.span(),
			Expression::FunctionCall(function_call) => function_call.span(),
			Expression::If(if_expression) => if_expression.span(),
			Expression::FieldAccess(field_access) => field_access.span(),
			Expression::ForEachLoop(for_each_loop) => for_each_loop.span(),
			Expression::Unary(unary) => unary.span(),
			Expression::List(list) => list.span(),
			Expression::Group(group) => group.span(),
			Expression::Either(either) => either.span(),
			Expression::Binary(binary_expression) => binary_expression.span(),
			Expression::Number(number) => number.span(),
			Expression::Text(text) => text.span(),
			Expression::Action(action) => action.span(),
			Expression::Extend(extend) => extend.span(),
			Expression::Parenthesized(parenthesized_expression) => parenthesized_expression.span(),
			Expression::ErrorExpression(error_expression) => error_expression.span(),
		}
	}
}
