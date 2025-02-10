use std::fmt::Debug;

// This is required because of a bug in `try_as`
use try_as::traits::{self as try_as_traits, TryAsMut};

use crate::{
	api::{context::context, scope::ScopeData, traits::TryAs as _},
	bail_err,
	cli::theme::Styled,
	comptime::{memory::VirtualPointer, CompileTime, CompileTimeError},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::Span,
	parser::{
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
		statements::tag::TagList,
		Parse,
		TokenQueue,
		TokenQueueFunctionality as _,
		TryParse,
	},
	transpiler::TranspileToC,
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

/// The `sugar` module. This module handles parsing expressions that are just syntactic sugar for
/// other constructs in the language, as opposed to their own syntax. For example, creating a
/// literal list such as `[1, 2, 3]` is just syntactic sugar for creating an instance of the `List`
/// group, and the result of parsing such an expression is simply an `ObjectConstructor`, as
/// opposed to being it's own type. Such syntaxes are parsed in this `sugar` module.
pub mod sugar;
pub mod unary;

#[derive(Clone, try_as::macros::From, try_as::macros::TryInto, try_as::macros::TryAsRef, try_as::macros::TryAsMut)]
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
	RepresentAs(Extend),
	ErrorExpression(Span),
}

impl Parse for Expression {
	type Output = Self;

	fn parse(tokens: &mut TokenQueue) -> Self::Output {
		let start = tokens.front().unwrap().span;
		let result = BinaryExpression::try_parse(tokens);
		match result {
			Ok(expression) => expression,
			Err(error) => {
				context().add_diagnostic(error);
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

	fn evaluate_at_compile_time(self) -> Self::Output {
		match self {
			Self::Block(block) => block.evaluate_at_compile_time(),
			Self::FieldAccess(field_access) => field_access.evaluate_at_compile_time(),
			Self::FunctionCall(function_call) => function_call.evaluate_at_compile_time(),
			Self::If(if_expression) => if_expression.evaluate_at_compile_time(),
			Self::RepresentAs(represent_as) => Expression::RepresentAs(represent_as.evaluate_at_compile_time()),
			Self::Name(name) => name.clone().evaluate_at_compile_time(),
			Self::ObjectConstructor(constructor) => constructor.evaluate_at_compile_time(),
			Self::Parameter(parameter) => Expression::Parameter(parameter.evaluate_at_compile_time()),
			Self::Unary(unary) => unary.evaluate_at_compile_time(),
			Self::ForEachLoop(for_loop) => for_loop.evaluate_at_compile_time(),
			Self::Run(run_expression) => Expression::Run(run_expression.evaluate_at_compile_time()),
			Self::Pointer(pointer) => Expression::Pointer(pointer.evaluate_at_compile_time()),
			Self::ErrorExpression(_) => self,
		}
	}
}

impl Expression {
	pub fn try_as_literal(&self) -> &'static LiteralObject {
		match self {
			Self::Pointer(pointer) => pointer.virtual_deref(),
			Self::Name(name) => name.clone().evaluate_at_compile_time().try_as_literal(),
			_ => VirtualPointer::ERROR.virtual_deref(),
		}
	}

	pub fn is_fully_known_at_compile_time(&self) -> bool {
		match self {
			Self::Pointer(_) => true,
			Self::Parameter(_) => true,
			Self::Name(name) => name.clone().evaluate_at_compile_time().is_fully_known_at_compile_time(),
			_ => false,
		}
	}

	pub fn is_error(&self) -> bool {
		matches!(self, Expression::ErrorExpression(_))
	}

	pub fn evaluate_as_type(self) -> Expression {
		match self {
			Self::Pointer(pointer) => Expression::Pointer(pointer),
			_ => self.evaluate_at_compile_time(),
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
			Self::RepresentAs(_) => "represent-as expression",
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

		bail_err! {
			base = format!("A value that's not fully known at compile-time was used as a type; It can only be evaluated into a {}", self.kind_name().bold().yellow()),
		};
	}

	pub fn is_true(&self) -> bool {
		let Ok(literal_address) = self.try_as::<VirtualPointer>() else {
			return false;
		};

		let true_address = context().scope_data.get_variable_from_id("true", ScopeData::get_stdlib_id()).unwrap().try_as().unwrap();

		literal_address == true_address
	}

	pub fn set_tags(&mut self, tags: TagList) {
		match self {
			Self::ObjectConstructor(constructor) => constructor.tags = tags,
			Self::Pointer(pointer) => pointer.virtual_deref_mut().tags = tags,
			Self::FunctionCall(function_call) => function_call.tags = tags,
			_ => {},
		};
	}

	pub fn try_set_name(&mut self, name: Name) {
		match self {
			Self::ObjectConstructor(object) => object.name = name,
			Self::Pointer(pointer) => {
				let value = pointer.virtual_deref_mut();
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
					let mut either = Either::from_literal(value).unwrap();
					either.set_name(name);
					*value = either.to_literal();
					value.address = address;
					return;
				}

				value.name = name;
			},
			_ => {},
		}
	}

	pub fn try_set_scope_label(&mut self, name: Name) {
		let scope_id = match self {
			Self::If(if_expression) => Some(if_expression.inner_scope_id()),
			Self::Pointer(pointer) => pointer.virtual_deref().inner_scope_id,
			_ => None,
		};

		if let Some(scope_id) = scope_id {
			context().scope_data.get_scope_mut_from_id(scope_id).set_label(name);
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
	pub fn is_assignable_to_type(&self, target_type: VirtualPointer) -> anyhow::Result<bool> {
		let this_type = self.get_type()?.virtual_deref();
		this_type.is_this_type_assignable_to_type(target_type)
	}
}

impl TranspileToC for Expression {
	fn to_c(&self) -> anyhow::Result<String> {
		Ok(match self {
			Self::If(if_expression) => if_expression.to_c()?,
			Self::Block(block) => block.to_c()?,
			Self::FieldAccess(field_access) => field_access.to_c()?,
			Self::Name(name) => name.to_c()?,
			Self::FunctionCall(function_call) => function_call.to_c()?,
			Self::ForEachLoop(for_each_loop) => for_each_loop.to_c()?,
			Self::Pointer(pointer) => pointer.to_c()?,
			Self::ObjectConstructor(object_constructor) => object_constructor.to_c()?,
			Self::Run(run_expression) => run_expression.to_c()?,
			Self::ErrorExpression(_) => "void".to_owned(),
			_ => todo!(),
		})
	}
}

impl Typed for Expression {
	fn get_type(&self) -> anyhow::Result<VirtualPointer> {
		Ok(match self {
			Expression::Pointer(pointer) => pointer.virtual_deref().get_type()?,
			Expression::FunctionCall(function_call) => function_call.get_type()?,
			Expression::Run(run_expression) => run_expression.get_type()?,
			Expression::Parameter(parameter) => parameter.get_type()?,
			Expression::ErrorExpression(_span) => bail_err! {
				base = "Attempted to get the type of a non-existent value",
				while = "getting the type of a generic expression",
			},
			value => {
				dbg!(value);
				todo!()
			},
		})
	}
}

impl Spanned for Expression {
	fn span(&self) -> Span {
		match self {
			Expression::Name(name) => name.span(),
			Expression::Run(run_expression) => run_expression.span(),
			Expression::Block(block) => block.span(),
			Expression::ObjectConstructor(object_constructor) => object_constructor.span(),
			Expression::Pointer(virtual_pointer) => virtual_pointer.span(),
			Expression::FunctionCall(function_call) => function_call.span(),
			Expression::If(if_expression) => if_expression.span(),
			Expression::FieldAccess(field_access) => field_access.span(),
			Expression::ForEachLoop(for_each_loop) => for_each_loop.span(),
			Expression::Parameter(parameter) => parameter.span(),
			Expression::RepresentAs(represent_as) => represent_as.span(),
			Expression::Unary(unary) => unary.span(),
			Expression::ErrorExpression(span) => *span,
		}
	}
}

pub trait Typed {
	fn get_type(&self) -> anyhow::Result<VirtualPointer>;
}

pub trait Spanned {
	/// Returns the section of the source code that this expression spans. This is used by the compiler to print information about
	/// errors that occur, such as while line and column the error occurred on.
	///
	/// # Returns
	///
	/// The second of the program's source code that this expression spans.
	fn span(&self) -> Span;
}

impl RuntimeableExpression for Expression {
	fn evaluate_subexpressions_at_compile_time(self) -> Self {
		match self {
			Self::FunctionCall(function_call) => Expression::FunctionCall(function_call.evaluate_subexpressions_at_compile_time()),
			_ => {
				context().add_diagnostic(Diagnostic {
					span: self.span(),
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::RunNonFunctionCall)),
				});
				self
			},
		}
	}
}

impl Debug for Expression {
	fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Block(block) => block.fmt(formatter),
			Self::FieldAccess(field_access) => field_access.fmt(formatter),
			Self::FunctionCall(function_call) => function_call.fmt(formatter),
			Self::ForEachLoop(for_loop) => for_loop.fmt(formatter),
			Self::If(if_expression) => if_expression.fmt(formatter),
			Self::Unary(unary) => unary.fmt(formatter),
			Self::Name(name) => name.fmt(formatter),
			Self::ObjectConstructor(object) => object.fmt(formatter),
			Self::Parameter(parameter) => parameter.fmt(formatter),
			Self::Pointer(pointer) => pointer.fmt(formatter),
			Self::Run(run) => run.fmt(formatter),
			Self::RepresentAs(represent_as) => represent_as.fmt(formatter),
			Self::ErrorExpression(_span) => write!(formatter, "{}", "<void>".style(context().theme.keyword())),
		}
	}
}
