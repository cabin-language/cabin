use std::fmt::Debug;

// This is required because of a bug in `try_as`
use try_as::traits::{self as try_as_traits, TryAsMut};

use crate::{
	api::{context::Context, scope::ScopeData, traits::TryAs as _},
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
			literal::{LiteralConvertible, LiteralObject},
			name::Name,
			object::ObjectConstructor,
			operators::BinaryExpression,
			parameter::Parameter,
			run::{RunExpression, RuntimeableExpression},
			unary::UnaryOperation,
		},
		misc::tag::TagList,
	},
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::Span,
	parser::{Parse, TokenQueue, TokenQueueFunctionality as _, TryParse as _},
	transpiler::{TranspileError, TranspileToC},
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
pub mod oneof;
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
	Pointer(VirtualPointer),
	Run(RunExpression),
	Unary(UnaryOperation),
	Parameter(Parameter),
	Extend(Extend),
	ErrorExpression(Span),
}

impl Parse for Expression {
	type Output = Self;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output {
		let start = tokens.front().unwrap().span;
		let result = BinaryExpression::try_parse(tokens, context);
		match result {
			Ok(expression) => expression,
			Err(error) => {
				context.add_diagnostic(error);
				if let Ok(token_type) = tokens.peek_type() {
					let _ = tokens.pop(token_type).unwrap();
				}
				let end = tokens.front().unwrap().span;
				Expression::ErrorExpression(start.to(end))
			},
		}
	}
}

impl CompileTime for Expression {
	type Output = Expression;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		match self {
			Self::Block(block) => block.evaluate_at_compile_time(context),
			Self::FieldAccess(field_access) => field_access.evaluate_at_compile_time(context),
			Self::FunctionCall(function_call) => function_call.evaluate_at_compile_time(context),
			Self::If(if_expression) => if_expression.evaluate_at_compile_time(context),
			Self::Extend(represent_as) => Expression::Extend(represent_as.evaluate_at_compile_time(context)),
			Self::Name(name) => name.clone().evaluate_at_compile_time(context),
			Self::ObjectConstructor(constructor) => constructor.evaluate_at_compile_time(context),
			Self::Parameter(parameter) => Expression::Parameter(parameter.evaluate_at_compile_time(context)),
			Self::Unary(unary) => unary.evaluate_at_compile_time(context),
			Self::ForEachLoop(for_loop) => for_loop.evaluate_at_compile_time(context),
			Self::Run(run_expression) => Expression::Run(run_expression.evaluate_at_compile_time(context)),
			Self::Pointer(pointer) => Expression::Pointer(pointer.evaluate_at_compile_time(context)),
			Self::ErrorExpression(_) => self,
		}
	}
}

impl TranspileToC for Expression {
	fn to_c(&self, context: &mut Context, output: Option<String>) -> Result<String, TranspileError> {
		Ok(match self {
			Expression::Block(block) => block.to_c(context, None)?,
			Expression::If(if_expression) => if_expression.to_c(context, None)?,
			Expression::Name(name) => name.to_c(context, output)?,
			Expression::Pointer(pointer) => pointer.to_c(context, output)?,
			Expression::Run(run_expression) => run_expression.to_c(context, output)?,
			Expression::FieldAccess(field_access) => field_access.to_c(context, output)?,
			Expression::ObjectConstructor(object_constructor) => object_constructor.to_c(context, output)?,

			// later...
			Expression::FunctionCall(function_call) => todo!(),
			Expression::ForEachLoop(for_each_loop) => todo!(),
			Expression::Unary(unary_operation) => todo!(),

			// Special cases
			Expression::Parameter(_parameter) => unreachable!(),
			Expression::Extend(_extend) => String::new(),
			Expression::ErrorExpression(_) => return Err(TranspileError::TranspileError),
		})
	}

	fn c_prelude(&self, context: &mut Context) -> Result<String, TranspileError> {
		match self {
			Expression::ObjectConstructor(object) => object.c_prelude(context),
			Expression::Pointer(pointer) => pointer.c_prelude(context),
			_ => Ok(String::new()),
		}
	}
}

impl Expression {
	pub fn try_as_literal<'ctx>(&self, context: &'ctx mut Context) -> &'ctx LiteralObject {
		match self {
			Self::Pointer(pointer) => pointer.virtual_deref(context),
			Self::Name(name) => name.clone().evaluate_at_compile_time(context).try_as_literal(context),
			_ => VirtualPointer::ERROR.virtual_deref(context),
		}
	}

	pub fn is_fully_known_at_compile_time(&self, context: &mut Context) -> bool {
		match self {
			Self::Pointer(_) => true,
			Self::Parameter(_) => true,
			Self::Name(name) => name.clone().evaluate_at_compile_time(context).is_fully_known_at_compile_time(context),
			_ => false,
		}
	}

	pub fn is_error(&self) -> bool {
		matches!(self, Expression::ErrorExpression(_))
	}

	pub fn evaluate_as_type(self, context: &mut Context) -> Expression {
		match self {
			Self::Pointer(pointer) => Expression::Pointer(pointer),
			_ => self.evaluate_at_compile_time(context),
		}
	}

	/// Returns whether this expression is a virtual pointer.
	pub const fn is_pointer(&self) -> bool {
		matches!(self, Self::Pointer(_))
	}

	/// Returns the name of this type of expression as a string.
	///
	/// This is used when the compiler reports errors; For example, if an if-expression is
	/// used as a type, which should be a literal, the compiler will say something like "attempted
	/// to parse a literal, but an if-expression was found".
	///
	/// # Returns
	/// The name of the kind of expression of this as a string.
	pub const fn kind_name(&self) -> &'static str {
		match self {
			Self::Block(_) => "block",
			Self::FieldAccess(_) => "field access",
			Self::FunctionCall(_) => "function call",
			Self::Name(_) => "name",
			Self::ObjectConstructor(_) => "object constructor",
			Self::Unary(_) => "unary operation",
			Self::ErrorExpression(_) => "non-existent value",
			Self::Pointer(_) => "pointer",
			Self::If(_) => "if expression",
			Self::ForEachLoop(_) => "for-each loop",
			Self::Run(_) => "run expression",
			Self::Parameter(_) => "parameter",
			Self::Extend(_) => "represent-as expression",
		}
	}

	/// Returns a new owned pointer to the same value in virtual memory as this referenced
	/// pointer. If this expression does indeed refer to a pointer, this is effectively a
	/// cheap `to_owned()`. If not, an error is returned.
	///
	/// # Errors
	///
	/// If this expression doesn't refer to a pointer.
	///
	/// # Performance
	///
	/// This clone is very cheap; Only the underlying pointer address (a `usize`) is cloned.
	pub fn try_clone_pointer(&self) -> anyhow::Result<Expression> {
		if let Self::Pointer(address) = self {
			return Ok(Expression::Pointer(*address));
		}

		anyhow::bail!(
			"A value that's not fully known at compile-time was used as a type; It can only be evaluated into a {}",
			self.kind_name()
		);
	}

	pub(crate) fn is_true(&self, context: &mut Context) -> bool {
		let Ok(literal_address) = self.try_as::<VirtualPointer>() else {
			return false;
		};

		let true_address = context.scope_tree.get_variable_from_id("true", ScopeData::get_stdlib_id()).unwrap().try_as().unwrap();

		literal_address == true_address
	}

	pub(crate) fn set_tags(&mut self, tags: TagList, context: &mut Context) {
		match self {
			Self::ObjectConstructor(constructor) => constructor.tags = tags,
			Self::Pointer(pointer) => pointer.virtual_deref_mut(context).tags = tags,
			Self::FunctionCall(function_call) => function_call.tags = tags,
			_ => {},
		};
	}

	pub(crate) fn try_set_name(&mut self, name: Name, context: &mut Context) {
		match self {
			Self::ObjectConstructor(object) => object.name = name,
			Self::Pointer(pointer) => {
				let value = pointer.virtual_deref_mut(context);
				let address = value.address;

				if value.type_name() == &"Group".into() {
					let mut group = GroupDeclaration::from_literal(value).unwrap();
					group.set_name(name);
					*value = group.to_literal();
					value.address = address;
					return;
				}

				if value.type_name() == &"RepresentAs".into() {
					let mut represent_as = Extend::from_literal(value).unwrap();
					represent_as.set_name(name);
					*value = represent_as.to_literal();
					value.address = address;
					return;
				}

				if value.type_name() == &"Either".into() {
					let either = Either::from_literal(value).unwrap();
					*value = either.to_literal();
					value.address = address;
					return;
				}

				value.name = name;
			},
			_ => {},
		}
	}

	pub fn try_set_scope_label(&mut self, name: Name, context: &mut Context) {
		let scope_id = match self {
			Self::If(if_expression) => Some(if_expression.inner_scope_id()),
			Self::Pointer(pointer) => pointer.virtual_deref(context).inner_scope_id,
			_ => None,
		};

		if let Some(scope_id) = scope_id {
			context.scope_tree.get_scope_mut_from_id(scope_id).set_label(name);
		}
	}

	/// Returns whether this expression can be assigned to the type pointed to by `target_type`, which is generally
	/// a call to `Typed::get_type()`.
	///
	/// # Parameters
	///
	/// - `target_type` - A pointer to the group declaration that represents the type we are trying to assign to.
	/// - `context` - Global data about the compiler state.
	///
	/// # Returns
	///
	/// whether this expression can be assigned to the given type.
	pub fn is_assignable_to_type(&self, target_type: VirtualPointer, context: &mut Context) -> anyhow::Result<bool> {
		let this_type = self.get_type(context)?.virtual_deref(context);
		this_type.is_this_type_assignable_to_type(target_type, context)
	}
}

impl Typed for Expression {
	fn get_type(&self, context: &mut Context) -> anyhow::Result<VirtualPointer> {
		Ok(match self {
			Expression::Pointer(pointer) => pointer.virtual_deref(context).to_owned().get_type(context)?,
			Expression::FunctionCall(function_call) => function_call.get_type(context)?,
			Expression::Run(run_expression) => run_expression.get_type(context)?,
			Expression::Parameter(parameter) => parameter.get_type(context)?,
			Expression::ErrorExpression(_span) => VirtualPointer::ERROR,
			value => {
				dbg!(value);
				todo!()
			},
		})
	}
}

impl Spanned for Expression {
	fn span(&self, context: &Context) -> Span {
		match self {
			Expression::Name(name) => name.span(context),
			Expression::Run(run_expression) => run_expression.span(context),
			Expression::Block(block) => block.span(context),
			Expression::ObjectConstructor(object_constructor) => object_constructor.span(context),
			Expression::Pointer(virtual_pointer) => virtual_pointer.span(context),
			Expression::FunctionCall(function_call) => function_call.span(context),
			Expression::If(if_expression) => if_expression.span(context),
			Expression::FieldAccess(field_access) => field_access.span(context),
			Expression::ForEachLoop(for_each_loop) => for_each_loop.span(context),
			Expression::Parameter(parameter) => parameter.span(context),
			Expression::Extend(represent_as) => represent_as.span(context),
			Expression::Unary(unary) => unary.span(context),
			Expression::ErrorExpression(span) => *span,
		}
	}
}

pub trait Typed {
	fn get_type(&self, context: &mut Context) -> anyhow::Result<VirtualPointer>;
}

pub trait Spanned {
	/// Returns the section of the source code that this expression spans. This is used by the compiler to print information about
	/// errors that occur, such as while line and column the error occurred on.
	///
	/// # Returns
	///
	/// The second of the program's source code that this expression spans.
	fn span(&self, context: &Context) -> Span;
}

impl RuntimeableExpression for Expression {
	fn evaluate_subexpressions_at_compile_time(self, context: &mut Context) -> Self {
		match self {
			Self::FunctionCall(function_call) => Expression::FunctionCall(function_call.evaluate_subexpressions_at_compile_time(context)),
			_ => {
				context.add_diagnostic(Diagnostic {
					span: self.span(context),
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::RunNonFunctionCall)),
				});
				self
			},
		}
	}
}
