use std::fmt::Debug;

use literal::UnevaluatedLiteral;
// This is required because of a bug in `try_as`
use try_as::traits::{self as try_as_traits, TryAsMut};

use crate::{
	api::context::Context,
	ast::{
		expressions::{
			block::Block,
			field_access::FieldAccess,
			foreach::ForEachLoop,
			function_call::FunctionCall,
			if_expression::IfExpression,
			literal::EvaluatedLiteral,
			name::Name,
			object::ObjectConstructor,
			operators::BinaryExpression,
			parameter::Parameter,
			run::{RunExpression, RuntimeableExpression},
			unary::UnaryOperation,
		},
		misc::tag::TagList,
		sugar::list::List,
	},
	comptime::{memory::ExpressionPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	interpreter::Runtime,
	io::{IoReader, IoWriter},
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
pub mod literal;
pub mod name;
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
	Literal(UnevaluatedLiteral),
	EvaluatedLiteral(EvaluatedLiteral),
	List(List),
}

impl Parse for Expression {
	type Output = ExpressionPointer;

	fn parse<Input: IoReader, Output: IoWriter, Error: IoWriter>(tokens: &mut TokenQueue, context: &mut Context<Input, Output, Error>) -> Self::Output {
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
pub enum ExpressionOrPointer {
	Expression(Expression),
	Pointer(ExpressionPointer),
}

impl CompileTime for Expression {
	type Output = ExpressionOrPointer;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		match self {
			Self::Block(block) => ExpressionOrPointer::Expression(Expression::Block(block.evaluate_at_compile_time(context))),
			Self::FieldAccess(field_access) => field_access.evaluate_at_compile_time(context),
			Self::FunctionCall(function_call) => function_call.evaluate_at_compile_time(context),
			Self::If(if_expression) => if_expression.evaluate_at_compile_time(context),
			Self::Name(name) => ExpressionOrPointer::Expression(Expression::Name(name.evaluate_at_compile_time(context))),
			Self::ObjectConstructor(constructor) => ExpressionOrPointer::Expression(constructor.evaluate_at_compile_time(context)),
			Self::ForEachLoop(for_loop) => for_loop.evaluate_at_compile_time(context),
			Self::Run(run_expression) => ExpressionOrPointer::Expression(Expression::Run(run_expression.evaluate_at_compile_time(context))),
			Self::EvaluatedLiteral(_) => ExpressionOrPointer::Expression(self),
			Self::Literal(literal) => ExpressionOrPointer::Expression(Expression::EvaluatedLiteral(literal.evaluate_at_compile_time(context))),
			Self::List(list) => ExpressionOrPointer::Expression(list.evaluate_at_compile_time(context)),
			Self::Unary(_) | Self::Parameter(_) => todo!(),
		}
	}
}

impl Runtime for Expression {
	type Output = ExpressionOrPointer;

	fn evaluate_at_runtime<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		match self {
			Self::Run(run_expression) => ExpressionOrPointer::Pointer(run_expression.evaluate_at_runtime(context)),
			expression => expression.evaluate_at_compile_time(context),
		}
	}
}

impl TranspileToC for Expression {
	fn to_c<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, _context: &mut Context<Input, Output, Error>, _output: Option<String>) -> Result<String, TranspileError> {
		todo!()
	}

	fn c_prelude<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Result<String, TranspileError> {
		match self {
			Expression::ObjectConstructor(object) => object.c_prelude(context),
			_ => Ok(String::new()),
		}
	}
}

impl Typed for Expression {
	fn get_type<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Type {
		match self {
			Expression::EvaluatedLiteral(literal) => literal.get_type(context),
			Expression::Name(name) => name.get_type(context),
			value => todo!("{value:?}"),
		}
	}
}

impl Expression {
	pub(crate) fn error<Input: IoReader, Output: IoWriter, Error: IoWriter>(span: Span, context: &mut Context<Input, Output, Error>) -> ExpressionPointer {
		Expression::EvaluatedLiteral(EvaluatedLiteral::ErrorLiteral(span)).store_in_memory(context)
	}

	pub(crate) fn store_in_memory<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> ExpressionPointer {
		context.virtual_memory.store(self)
	}

	pub(crate) fn set_tags(&mut self, tags: TagList) {
		match self {
			Expression::Literal(literal) => match literal {
				UnevaluatedLiteral::FunctionDeclaration(function) => function.set_tags(tags),
				_ => {},
			},
			_ => {},
		}
	}

	pub(crate) fn set_name(&mut self, name: Name) {
		match self {
			Expression::Literal(literal) => match literal {
				UnevaluatedLiteral::Group(function) => function.name = Some(name),
				_ => {},
			},
			_ => {},
		}
	}

	pub(crate) fn set_documentation(&mut self, documentation: &str) {
		match self {
			Expression::Literal(literal) => match literal {
				UnevaluatedLiteral::FunctionDeclaration(function) => function.documentation = Some(documentation.to_owned()),
				_ => {},
			},
			_ => {},
		}
	}

	pub fn get_documentation(&self) -> Option<&str> {
		match self {
			Expression::Literal(literal) => match literal {
				UnevaluatedLiteral::FunctionDeclaration(function) => function.documentation.as_ref().map(|doc| doc.as_str()),
				_ => None,
			},
			Self::EvaluatedLiteral(literal) => match literal {
				EvaluatedLiteral::FunctionDeclaration(function) => function.documentation.as_ref().map(|doc| doc.as_str()),
				_ => None,
			},
			_ => None,
		}
	}

	pub(crate) fn kind_name(&self) -> &'static str {
		match self {
			Expression::Block(_) => "block",
			Expression::FieldAccess(_) => "field access",
			Expression::FunctionCall(_) => "function call",
			Expression::If(_) => "if expression",
			Expression::Name(_) => "name",
			Expression::ObjectConstructor(_) => "object constructor",
			Expression::ForEachLoop(_) => "for loop",
			Expression::Run(_) => "run expression",
			Expression::Unary(_) => "unary operation",
			Expression::Parameter(_) => "parameter",
			Expression::EvaluatedLiteral(literal) => literal.kind_name(),
			Expression::Literal(literal) => literal.kind_name(),
			Expression::List(_) => "list",
		}
	}
}

impl Spanned for Expression {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &Context<Input, Output, Error>) -> Span {
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
			Expression::Unary(unary) => unary.span(context),
			Expression::EvaluatedLiteral(literal) => literal.span(context),
			Expression::Literal(literal) => literal.span(context),
			Expression::List(list) => list.span(context),
		}
	}
}

impl RuntimeableExpression for Expression {
	fn evaluate_subexpressions_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self {
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
