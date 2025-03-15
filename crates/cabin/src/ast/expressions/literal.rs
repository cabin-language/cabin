use std::collections::HashMap;

use try_as::traits as try_as_traits;

use super::{either::EvaluatedEither, function_declaration::FunctionDeclaration, group::GroupDeclaration};
use crate::{
	ast::{
		expressions::{
			either::Either,
			extend::{EvaluatedExtend, Extend},
			field_access::Dot,
			function_declaration::EvaluatedFunctionDeclaration,
			group::EvaluatedGroupDeclaration,
			name::Name,
			Expression,
		},
		sugar::{list::LiteralList, string::CabinString},
	},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTime,
		CompileTimeError,
	},
	diagnostics::Diagnostic,
	io::{IoReader, IoWriter},
	typechecker::{Type, Typed},
	Context,
	Span,
	Spanned,
};

#[derive(Clone, Debug)]
pub enum Literal<'context> {
	Evaluated(&'context EvaluatedLiteral),
	Unevaluated(&'context UnevaluatedLiteral),
}

impl Literal<'_> {
	pub const fn as_evaluated(&self) -> Option<&EvaluatedLiteral> {
		match self {
			Literal::Evaluated(evaluated) => Some(evaluated),
			Literal::Unevaluated(_) => None,
		}
	}
}

pub enum LiteralMut<'context> {
	Evaluated(&'context mut EvaluatedLiteral),
	Unevaluated(&'context mut UnevaluatedLiteral),
}

#[derive(Debug, Clone, try_as::macros::TryAsRef)]
pub enum UnevaluatedLiteral {
	String(CabinString),
	FunctionDeclaration(FunctionDeclaration),
	Group(GroupDeclaration),
	Extend(Extend),
	Either(Either),
}

impl UnevaluatedLiteral {
	pub(crate) const fn kind_name(&self) -> &'static str {
		match self {
			Self::Group(_) => "Group",
			Self::FunctionDeclaration(_) => "Function",
			Self::Extend(_) => "Extension",
			Self::String(_) => "String",
			Self::Either(_) => "Either",
		}
	}
}

impl CompileTime for UnevaluatedLiteral {
	type Output = EvaluatedLiteral;

	fn evaluate_at_compile_time<Input: IoReader, Output: IoWriter, Error: IoWriter>(self, context: &mut Context<Input, Output, Error>) -> Self::Output {
		match self {
			Self::FunctionDeclaration(function) => EvaluatedLiteral::FunctionDeclaration(function.evaluate_at_compile_time(context)),
			Self::Either(either) => EvaluatedLiteral::Either(either.evaluate_at_compile_time(context)),
			Self::Extend(extend) => EvaluatedLiteral::Extend(extend.evaluate_at_compile_time(context)),
			Self::Group(group) => EvaluatedLiteral::Group(group.evaluate_at_compile_time(context)),
			Self::String(string) => EvaluatedLiteral::String(string),
		}
	}
}

impl Spanned for UnevaluatedLiteral {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &Context<Input, Output, Error>) -> Span {
		match self {
			Self::String(string) => string.span(context),
			Self::FunctionDeclaration(function) => function.span(context),
			Self::Group(group) => group.span(context),
			Self::Extend(extend) => extend.span(context),
			Self::Either(either) => either.span(context),
		}
	}
}

#[derive(Debug, Clone, try_as::macros::TryAsRef)]
pub enum EvaluatedLiteral {
	Object(Object),
	String(CabinString),
	Number(f64),
	List(LiteralList),
	FunctionDeclaration(EvaluatedFunctionDeclaration),
	Group(EvaluatedGroupDeclaration),
	Extend(EvaluatedExtend),
	Either(EvaluatedEither),
	ErrorLiteral(Span),
}

impl EvaluatedLiteral {
	pub(crate) const fn kind_name(&self) -> &'static str {
		match self {
			Self::Group(_) => "Group",
			Self::Object(_) => "Object",
			Self::FunctionDeclaration(_) => "Function",
			Self::Extend(_) => "Extension",
			Self::List(_) => "List",
			Self::String(_) => "String",
			Self::Number(_) => "Number",
			Self::Either(_) => "Either",
			Self::ErrorLiteral(_) => "Error",
		}
	}
}

impl Typed for EvaluatedLiteral {
	fn get_type<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &mut Context<Input, Output, Error>) -> Type {
		match self {
			Self::String(_) => Type::Literal(context.scope_tree.get_builtin("Text").unwrap().try_as_literal(context).unwrap_or(LiteralPointer::ERROR)),
			Self::Number(_) => Type::Literal(context.scope_tree.get_builtin("Number").unwrap().try_as_literal(context).unwrap_or(LiteralPointer::ERROR)),
			Self::ErrorLiteral(_) => Type::Literal(LiteralPointer::ERROR),
			EvaluatedLiteral::FunctionDeclaration(_) => Type::Literal(Expression::EvaluatedLiteral(self.to_owned()).store_in_memory(context).as_literal(context)),
			literal => todo!("{literal:?}"),
		}
	}
}

impl Dot for EvaluatedLiteral {
	fn dot<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, name: &Name, context: &mut Context<Input, Output, Error>) -> ExpressionPointer {
		match self {
			EvaluatedLiteral::Object(object) => object.dot(name, context),
			EvaluatedLiteral::Either(either) => either.dot(name, context),
			EvaluatedLiteral::String(string) => string.dot(name, context),
			EvaluatedLiteral::ErrorLiteral(_) => Expression::EvaluatedLiteral(self.to_owned()).store_in_memory(context),
			value => todo!("{value:?}"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Object {
	pub(crate) span: Span,
	pub(crate) type_name: Name,
	pub(crate) fields: HashMap<Name, LiteralPointer>,
}

impl Object {
	pub(crate) fn empty() -> Object {
		Self {
			type_name: Name::hardcoded("Object"),
			fields: HashMap::new(),
			span: Span::unknown(),
		}
	}

	pub(crate) const fn type_name(&self) -> &Name {
		&self.type_name
	}

	pub(crate) fn get_field<StringLike: AsRef<str>>(&self, name: StringLike) -> Option<LiteralPointer> {
		let name = name.as_ref();
		self.fields
			.iter()
			.find_map(|(field_name, field_value)| (field_name.unmangled_name() == name).then_some(field_value.to_owned()))
	}
}

impl Dot for Object {
	fn dot<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, name: &Name, context: &mut Context<Input, Output, Error>) -> ExpressionPointer {
		self.fields
			.get(name)
			.unwrap_or_else(|| {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					info: CompileTimeError::NoSuchField(name.unmangled_name().to_owned()).into(),
					span: self.span,
				});
				&LiteralPointer::ERROR
			})
			.to_owned()
			.into()
	}
}

impl Spanned for EvaluatedLiteral {
	fn span<Input: IoReader, Output: IoWriter, Error: IoWriter>(&self, context: &Context<Input, Output, Error>) -> Span {
		match self {
			Self::String(string) => string.span(context),
			_ => Span::unknown(),
		}
	}
}
