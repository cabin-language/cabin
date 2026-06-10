use std::collections::HashMap;

use try_as::traits as try_as_traits;

use super::{action::Action, either::EvaluatedEither, group::Group};
use crate::{
	Context,
	Span,
	Spanned,
	ast::{
		expressions::{
			Expression,
			action::EvaluatedAction,
			either::Either,
			extend::{EvaluatedExtend, Extend},
			field_access::DoubleColon,
			group::EvaluatedGroup,
			identifier::Identifier,
		},
		sugar::{list::LiteralList, string::Text},
	},
	comptime::{
		CompileTime,
		CompileTimeError,
		memory::{ExpressionPointer, LiteralPointer},
	},
	diagnostics::Diagnostic,
	typechecker::{Type, Typed},
};

#[derive(Clone, Debug)]
pub enum LiteralRef<'context> {
	Evaluated(&'context EvaluatedLiteral),
	Unevaluated(&'context UnevaluatedLiteral),
}

impl LiteralRef<'_> {
	pub const fn as_evaluated(&self) -> Option<&EvaluatedLiteral> {
		match self {
			LiteralRef::Evaluated(evaluated) => Some(evaluated),
			LiteralRef::Unevaluated(_) => None,
		}
	}
}

pub enum LiteralMut<'context> {
	Evaluated(&'context mut EvaluatedLiteral),
	Unevaluated(&'context mut UnevaluatedLiteral),
}

#[derive(Debug, Clone, try_as::macros::TryAsRef)]
pub enum UnevaluatedLiteral {
	Text(Text),
	Action(Action),
	Group(Group),
	Extend(Extend),
	Either(Either),
}

impl UnevaluatedLiteral {
	pub const fn kind_name(&self) -> &'static str {
		match self {
			Self::Group(_) => "Group",
			Self::Action(_) => "Action",
			Self::Extend(_) => "Extension",
			Self::Text(_) => "Text",
			Self::Either(_) => "Either",
		}
	}
}

impl CompileTime for UnevaluatedLiteral {
	type Output = EvaluatedLiteral;

	fn evaluate_at_compile_time(self, context: &mut Context) -> Self::Output {
		match self {
			Self::Action(function) => EvaluatedLiteral::Action(function.evaluate_at_compile_time(context)),
			Self::Either(either) => EvaluatedLiteral::Either(either.evaluate_at_compile_time(context)),
			Self::Extend(extend) => EvaluatedLiteral::Extend(extend.evaluate_at_compile_time(context)),
			Self::Group(group) => EvaluatedLiteral::Group(group.evaluate_at_compile_time(context)),
			Self::Text(string) => EvaluatedLiteral::Text(string),
		}
	}
}

impl Spanned for UnevaluatedLiteral {
	fn span(&self, context: &Context) -> Span {
		match self {
			Self::Text(string) => string.span(context),
			Self::Action(function) => function.span(context),
			Self::Group(group) => group.span(context),
			Self::Extend(extend) => extend.span(context),
			Self::Either(either) => either.span(context),
		}
	}
}

#[derive(Debug, Clone, try_as::macros::TryAsRef)]
pub enum EvaluatedLiteral {
	Object(Object),
	Text(Text),
	Number(f64),
	List(LiteralList),
	Action(EvaluatedAction),
	Group(EvaluatedGroup),
	Extend(EvaluatedExtend),
	Either(EvaluatedEither),
	Error(Span),
}

impl EvaluatedLiteral {
	pub const fn kind_name(&self) -> &'static str {
		match self {
			Self::Group(_) => "Group",
			Self::Object(_) => "Object",
			Self::Action(_) => "Function",
			Self::Extend(_) => "Extension",
			Self::List(_) => "List",
			Self::Text(_) => "String",
			Self::Number(_) => "Number",
			Self::Either(_) => "Either",
			Self::Error(_) => "Error",
		}
	}
}

impl Typed for EvaluatedLiteral {
	fn get_type(&self, context: &mut Context) -> Type {
		match self {
			Self::Text(_) => Type::Literal(context.scope.get_builtin("Text").unwrap().try_as_literal(context).unwrap_or(LiteralPointer::ERROR)),
			Self::Number(_) => Type::Literal(context.scope.get_builtin("Number").unwrap().try_as_literal(context).unwrap_or(LiteralPointer::ERROR)),
			Self::Error(_) => Type::Literal(LiteralPointer::ERROR),
			EvaluatedLiteral::Action(_) => Type::Literal(Expression::EvaluatedLiteral(self.to_owned()).store_in_memory(context).as_literal(context)),
			literal => todo!("{literal:?}"),
		}
	}
}

impl DoubleColon for EvaluatedLiteral {
	fn double_colon(&self, name: &Identifier, context: &mut Context) -> ExpressionPointer {
		match self {
			EvaluatedLiteral::Object(object) => object.double_colon(name, context),
			EvaluatedLiteral::Either(either) => either.double_colon(name, context),
			EvaluatedLiteral::Text(string) => string.double_colon(name, context),
			EvaluatedLiteral::Error(_) => Expression::EvaluatedLiteral(self.to_owned()).store_in_memory(context),
			value => todo!("{value:?}"),
		}
	}
}

#[derive(Debug, Clone)]
pub struct Object {
	pub span: Span,
	pub type_name: Identifier,
	pub fields: HashMap<Identifier, LiteralPointer>,
}

impl Object {
	pub fn synthetic(type_name: Identifier, fields: HashMap<Identifier, LiteralPointer>, span: Span) -> Object {
		Object { type_name, fields, span }
	}

	pub const fn type_name(&self) -> &Identifier {
		&self.type_name
	}

	pub fn get_field<StringLike: AsRef<str>>(&self, name: StringLike) -> Option<LiteralPointer> {
		let name = name.as_ref();
		self.fields
			.iter()
			.find_map(|(field_name, field_value)| (field_name.source_identifier() == name).then_some(field_value.to_owned()))
	}
}

impl DoubleColon for Object {
	fn double_colon(&self, name: &Identifier, context: &mut Context) -> ExpressionPointer {
		self.fields
			.get(name)
			.unwrap_or_else(|| {
				context.add_diagnostic(Diagnostic {
					file: context.file.clone(),
					info: CompileTimeError::NoSuchField(name.source_identifier().to_owned()).into(),
					span: self.span,
				});
				&LiteralPointer::ERROR
			})
			.to_owned()
			.into()
	}
}

impl Spanned for EvaluatedLiteral {
	fn span(&self, context: &Context) -> Span {
		match self {
			Self::Text(string) => string.span(context),
			_ => Span::none(),
		}
	}
}
