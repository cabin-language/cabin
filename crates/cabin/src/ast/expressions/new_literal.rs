use std::collections::HashMap;

use try_as::traits as try_as_traits;

use super::{either::Either, extend::EvaluatedExtend, field_access::Dot, function_declaration::EvaluatedFunctionDeclaration, group::EvaluatedGroupDeclaration, name::Name};
use crate::{
	ast::sugar::{list::LiteralList, string::CabinString},
	comptime::{
		memory::{ExpressionPointer, LiteralPointer},
		CompileTimeError,
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	typechecker::{Type, Typed},
	Context,
	Span,
	Spanned,
};

#[derive(Debug, Clone, try_as::macros::TryAsRef)]
pub enum Literal {
	Object(Object),
	String(CabinString),
	Number(f64),
	List(LiteralList),
	FunctionDeclaration(EvaluatedFunctionDeclaration),
	Group(EvaluatedGroupDeclaration),
	Extend(EvaluatedExtend),
	Either(Either),
}

impl Literal {
	pub(crate) fn kind_name(&self) -> &'static str {
		match self {
			Self::Group(_) => "Group",
			Self::Object(_) => "Object",
			Self::FunctionDeclaration(_) => "Function",
			Self::Extend(_) => "Extension",
			Self::List(_) => "List",
			Self::String(_) => "String",
			Self::Number(_) => "Number",
			Self::Either(_) => "Either",
		}
	}
}

impl Typed for Literal {
	fn get_type(&self, context: &mut Context) -> Type {
		match self {
			Self::String(_) => Type::Literal(context.scope_tree.get_builtin("Text").unwrap().as_literal(context)),
			literal => todo!("{literal:?}"),
		}
	}
}

impl Dot for Literal {
	fn dot(&self, name: &Name, context: &mut Context) -> ExpressionPointer {
		match self {
			Literal::Object(object) => object.dot(name, context),
			Literal::Either(either) => either.dot(name, context),
			value => todo!("{value:?}"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Object {
	pub(crate) type_name: Name,
	pub(crate) fields: HashMap<Name, LiteralPointer>,
}

impl Object {
	pub(crate) fn empty() -> Object {
		Self {
			type_name: Name::hardcoded("Object"),
			fields: HashMap::new(),
		}
	}

	pub(crate) fn type_name(&self) -> &Name {
		&self.type_name
	}

	pub(crate) fn get_field<S: AsRef<str>>(&self, name: S) -> Option<LiteralPointer> {
		let name = name.as_ref();
		self.fields
			.iter()
			.find_map(|(field_name, field_value)| (field_name.unmangled_name() == name).then_some(field_value.to_owned()))
	}
}

impl Dot for Object {
	fn dot(&self, name: &Name, context: &mut Context) -> ExpressionPointer {
		self.fields
			.get(name)
			.unwrap_or_else(|| {
				context.add_diagnostic(Diagnostic {
					info: DiagnosticInfo::Error(crate::Error::CompileTime(CompileTimeError::NoSuchField(name.unmangled_name().to_owned()))),
					span: Span::unknown(),
				});
				&LiteralPointer::ERROR
			})
			.to_owned()
			.into()
	}
}

impl Spanned for Literal {
	fn span(&self, context: &Context) -> Span {
		match self {
			Self::String(string) => string.span(context),
			_ => Span::unknown(),
		}
	}
}
