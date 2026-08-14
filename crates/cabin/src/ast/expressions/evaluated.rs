use try_as::{
	macros::{TryAsMut, TryAsRef},
	traits as try_as_traits,
	traits::TryAsMut,
};

use crate::{
	Context,
	SpanLike,
	Spanned,
	ast::expressions::{
		action::EvaluatedAction,
		block::EvaluatedBlock,
		either::EvaluatedEither,
		error::ErrorExpression,
		extend::EvaluatedExtend,
		field_access::EvaluatedFieldAccess,
		foreach::EvaluatedForLoop,
		function_call::EvaluatedFunctionCall,
		group::EvaluatedGroup,
		identifier::Identifier,
		if_expression::EvaluatedIfExpression,
		list::EvaluatedList,
		new::EvaluatedNewExpression,
		number::Number,
		parenthesized::EvaluatedParenthesizedExpression,
		run::EvaluatedRunExpression,
	},
	comptime::{
		Dot,
		storage::{Evaluated, Pointer},
	},
};

#[derive(Debug, Clone, TryAsRef, TryAsMut)]
pub enum EvaluatedExpression {
	Action(EvaluatedAction),
	Either(EvaluatedEither),
	Group(EvaluatedGroup),
	Number(Number),

	Identifier(Identifier),
	List(EvaluatedList),
	New(EvaluatedNewExpression),
	If(EvaluatedIfExpression),
	Block(EvaluatedBlock),
	FieldAccess(EvaluatedFieldAccess),
	Parenthesized(EvaluatedParenthesizedExpression),
	ForLoop(EvaluatedForLoop),
	Run(Box<EvaluatedRunExpression>),
	ErrorExpression(ErrorExpression),
	Extend(EvaluatedExtend),
	FunctionCall(EvaluatedFunctionCall),
}

impl EvaluatedExpression {
	pub fn store(self, context: &mut Context) -> Pointer<EvaluatedExpression> {
		context.storage.store(self)
	}

	pub fn is_fully_known(&self, context: &Context) -> bool {
		todo!()
	}

	pub const fn kind_name(&self) -> &'static str {
		match self {
			EvaluatedExpression::Block(_) => "block",
			EvaluatedExpression::Action(_) => "action",
			EvaluatedExpression::Extend(_) => "extension",
			EvaluatedExpression::FieldAccess(_) => "field access",
			EvaluatedExpression::If(_) => "if expression",
			EvaluatedExpression::Identifier(_) => "name",
			EvaluatedExpression::New(_) => "object constructor",
			EvaluatedExpression::Run(_) => "run expression",
			EvaluatedExpression::List(_) => "list",
			EvaluatedExpression::Group(_) => "group",
			EvaluatedExpression::Either(_) => "either",
			EvaluatedExpression::FunctionCall(_) => "action call",
			EvaluatedExpression::Number(_) => "number",
			EvaluatedExpression::ForLoop(_) => "for loop",
			EvaluatedExpression::ErrorExpression(_) => "error",
			EvaluatedExpression::Parenthesized(_) => "parenthesized expression",
		}
	}
}

impl Spanned for EvaluatedExpression {
	fn span(&self) -> impl SpanLike {
		match self {
			EvaluatedExpression::If(if_expression) => if_expression.span().owned(),
			EvaluatedExpression::New(new) => new.span().owned(),
			EvaluatedExpression::Identifier(ident) => ident.span().owned(),
			EvaluatedExpression::Group(group) => group.span().owned(),
			EvaluatedExpression::Block(block) => block.span().owned(),
			EvaluatedExpression::ForLoop(for_loop) => for_loop.span().owned(),
			EvaluatedExpression::FieldAccess(field_access) => field_access.span().owned(),
			EvaluatedExpression::Run(run) => run.span().owned(),
			EvaluatedExpression::Either(either) => either.span().owned(),
			EvaluatedExpression::Number(number) => number.span().owned(),
			EvaluatedExpression::List(list) => list.span().owned(),
			EvaluatedExpression::Extend(extend) => extend.span().owned(),
			EvaluatedExpression::ErrorExpression(error_expression) => error_expression.span().owned(),
			EvaluatedExpression::Action(action) => action.span().owned(),
			EvaluatedExpression::FunctionCall(action_call) => action_call.span().owned(),
			EvaluatedExpression::Parenthesized(parenthesized) => parenthesized.span().owned(),
		}
	}
}

impl Dot for EvaluatedExpression {
	fn dot(&self, context: &Context, name: &str) -> Option<Pointer<Evaluated>> {
		match self {
			EvaluatedExpression::Either(either) => either.dot(context, name),
			EvaluatedExpression::List(list) => list.dot(context, name),

			EvaluatedExpression::Action(action) => todo!(),
			EvaluatedExpression::Group(group) => todo!(),
			EvaluatedExpression::Number(number) => todo!(),
			EvaluatedExpression::Identifier(identifier) => todo!(),
			EvaluatedExpression::New(new_expression) => todo!(),
			EvaluatedExpression::If(if_expression) => todo!(),
			EvaluatedExpression::Block(block) => todo!(),
			EvaluatedExpression::FieldAccess(field_access) => todo!(),
			EvaluatedExpression::Parenthesized(parenthesized_expression) => todo!(),
			EvaluatedExpression::ForLoop(for_loop) => todo!(),
			EvaluatedExpression::Run(run_expression) => todo!(),
			EvaluatedExpression::ErrorExpression(error_expression) => todo!(),
			EvaluatedExpression::Extend(extend) => todo!(),
			EvaluatedExpression::FunctionCall(function_call) => todo!(),
		}
	}
}
