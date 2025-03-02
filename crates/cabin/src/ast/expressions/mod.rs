use std::fmt::Debug;

use function_declaration::FunctionDeclaration;
use new_literal::Literal;
// This is required because of a bug in `try_as`
use try_as::traits::{self as try_as_traits, TryAsMut};

use super::misc::tag::TagList;
use crate::{
	api::context::Context,
	ast::{
		expressions::{
			block::Block,
			either::Either,
			extend::Extend,
			field_access::FieldAccess,
			foreach::ForEachLoop,
			function_call::FunctionCall,
			group::GroupDeclaration,
			if_expression::IfExpression,
			name::Name,
			object::ObjectConstructor,
			operators::BinaryExpression,
			parameter::Parameter,
			run::{RunExpression, RuntimeableExpression},
			unary::UnaryOperation,
		},
		sugar::list::List,
	},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
		CompileTimeError,
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	parser::{Parse, TokenQueue, TokenQueueFunctionality as _, TryParse as _},
	transpiler::{TranspileError, TranspileToC},
	typechecker::{Type, Typed},
	Span,
	Spanned,
};

pub mod block;
pub mod either;
pub mod extend;
pub mod field_access;
pub mod foreach;
pub mod function_call;
pub mod function_declaration;
pub mod group;
pub mod if_expression;
pub mod name;
pub mod new_literal;
pub mod object;
pub mod operators;
pub mod parameter;
pub mod run;
pub mod unary;

#[derive(Debug, Clone, try_as::macros::From, try_as::macros::TryInto, try_as::macros::TryAsRef, try_as::macros::TryAsMut)]
pub enum Expression {
	Block(Block),
	FieldAccess(FieldAccess),
	FunctionCall(FunctionCall),
	If(IfExpression),
	Name(Name),
	ObjectConstructor(ObjectConstructor),
	ForEachLoop(ForEachLoop),
	Run(RunExpression),
	Unary(UnaryOperation),
	Parameter(Parameter),
	Extend(Extend),
	Group(GroupDeclaration),
	FunctionDeclaration(FunctionDeclaration),
	Either(Either),
	Literal(Literal),
	List(List),
}

impl Parse for Expression {
	type Output = ExpressionPointer;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output {
		let start = tokens.front().unwrap().span;
		let result = BinaryExpression::try_parse(tokens, context);
		match result {
			Ok(expression) => expression,
			Err(error) => {
				context.add_diagnostic(error);
				if let Ok(token_type) = tokens.peek_type(context) {
					let _ = tokens.pop(token_type, context).unwrap();
				}
				let end = tokens.front().unwrap().span;
				Expression::error(start.to(end), context)
			},
		}
	}
}

#[derive(Debug)]
pub(crate) enum ExpressionOrPointer {
	Expression(Expression),
	Pointer(ExpressionPointer),
}

impl CompileTime for Expression {
	type Output = ExpressionOrPointer;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		match self {
			Self::Block(block) => ExpressionOrPointer::Expression(Expression::Block(block.evaluate_at_compile_time(context))),
			Self::FieldAccess(field_access) => field_access.evaluate_at_compile_time(context),
			Self::FunctionCall(function_call) => function_call.evaluate_at_compile_time(context),
			Self::If(if_expression) => if_expression.evaluate_at_compile_time(context),
			Self::Extend(extend) => ExpressionOrPointer::Expression(Expression::Literal(Literal::Extend(extend.evaluate_at_compile_time(context)))),
			Self::Name(name) => ExpressionOrPointer::Expression(Expression::Name(name.clone().evaluate_at_compile_time(context))),
			Self::ObjectConstructor(constructor) => ExpressionOrPointer::Expression(constructor.evaluate_at_compile_time(context)),
			Self::ForEachLoop(for_loop) => for_loop.evaluate_at_compile_time(context),
			Self::Run(run_expression) => ExpressionOrPointer::Expression(Expression::Run(run_expression.evaluate_at_compile_time(context))),
			Expression::Group(group_declaration) => ExpressionOrPointer::Expression(Expression::Literal(Literal::Group(group_declaration.evaluate_at_compile_time(context)))),
			Expression::FunctionDeclaration(function_declaration) => {
				ExpressionOrPointer::Expression(Expression::Literal(Literal::FunctionDeclaration(function_declaration.evaluate_at_compile_time(context))))
			},
			Expression::Either(either) => ExpressionOrPointer::Expression(Expression::Literal(Literal::Either(either.evaluate_at_compile_time(context)))),
			Self::Literal(_) => ExpressionOrPointer::Expression(self),
			Self::List(list) => ExpressionOrPointer::Expression(list.evaluate_at_compile_time(context)),
			Self::Unary(_) | Self::Parameter(_) => todo!(),
		}
	}
}

impl TranspileToC for Expression {
	fn to_c(&self, _context: &mut Context, _output: Option<String>) -> Result<String, TranspileError> {
		todo!()
	}

	fn c_prelude(&self, context: &mut Context) -> Result<String, TranspileError> {
		match self {
			Expression::ObjectConstructor(object) => object.c_prelude(context),
			_ => Ok(String::new()),
		}
	}
}

impl Typed for Expression {
	fn get_type(&self, context: &mut Context) -> Type {
		match self {
			Expression::Literal(literal) => literal.get_type(context),
			Expression::Name(name) => name.get_type(context),
			value => todo!("{value:?}"),
		}
	}
}

impl Expression {
	pub(crate) fn error(span: Span, context: &mut Context) -> ExpressionPointer {
		Expression::Literal(Literal::ErrorLiteral(span)).store_in_memory(context)
	}

	pub(crate) fn store_in_memory(self, context: &mut Context) -> ExpressionPointer {
		context.virtual_memory.store(self)
	}

	pub(crate) fn set_tags(&mut self, tags: TagList) {
		match self {
			Self::FunctionDeclaration(function) => function.set_tags(tags),
			_ => {},
		}
	}
}

impl Spanned for Expression {
	fn span(&self, context: &Context) -> Span {
		match self {
			Expression::Name(name) => name.span(context),
			Expression::Run(run_expression) => run_expression.span(context),
			Expression::Block(block) => block.span(context),
			Expression::ObjectConstructor(object_constructor) => object_constructor.span(context),
			Expression::FunctionCall(function_call) => function_call.span(context),
			Expression::If(if_expression) => if_expression.span(context),
			Expression::FieldAccess(field_access) => field_access.span(context),
			Expression::ForEachLoop(for_each_loop) => for_each_loop.span(context),
			Expression::Parameter(parameter) => parameter.span(context),
			Expression::Extend(represent_as) => represent_as.span(context),
			Expression::Unary(unary) => unary.span(context),
			Expression::Literal(literal) => literal.span(context),
			Expression::Group(group) => group.span(context),
			Expression::Either(either) => either.span(context),
			Expression::FunctionDeclaration(function) => function.span(context),
			Expression::List(list) => list.span(context),
		}
	}
}

impl RuntimeableExpression for Expression {
	fn evaluate_subexpressions_at_compile_time(self, context: &mut Context) -> Self {
		match self {
			Self::FunctionCall(function_call) => Expression::FunctionCall(function_call.evaluate_subexpressions_at_compile_time(context)),
			_ => {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					span: self.span(context),
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::RunNonFunctionCall)),
				});
				self
			},
		}
	}
}

#[derive(Debug, Clone, try_as::macros::From, try_as::macros::TryInto, try_as::macros::TryAsRef, try_as::macros::TryAsMut)]
pub enum CompileTimeEvaluatedExpression {
	Literal(Literal),
}
