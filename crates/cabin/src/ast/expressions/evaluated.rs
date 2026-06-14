use try_as::{macros::TryAsRef, traits as try_as_traits};

use crate::{
	Context,
	Span,
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
	comptime::memory::{Pointer, Storable},
};

#[derive(Debug, Clone, TryAsRef)]
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
	pub fn store(self, context: &mut Context) -> Pointer {
		context.memory.store(Storable::Expression(self))
	}

	pub const fn is_fully_known(&self) -> bool {
		matches!(self, Self::Action(_) | Self::Either(_) | Self::Group(_) | Self::Number(_))
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
	fn span(&self) -> Span {
		match self {
			EvaluatedExpression::If(if_expression) => if_expression.span(),
			EvaluatedExpression::New(new) => new.span(),
			EvaluatedExpression::Identifier(ident) => ident.span(),
			EvaluatedExpression::Group(group) => group.span(),
			EvaluatedExpression::Block(block) => block.span(),
			EvaluatedExpression::ForLoop(for_loop) => for_loop.span(),
			EvaluatedExpression::FieldAccess(field_access) => field_access.span(),
			EvaluatedExpression::Run(run) => run.span(),
			EvaluatedExpression::Either(either) => either.span(),
			EvaluatedExpression::Number(number) => number.span(),
			EvaluatedExpression::List(list) => list.span(),
			EvaluatedExpression::Extend(extend) => extend.span(),
			EvaluatedExpression::ErrorExpression(error_expression) => error_expression.span(),
			EvaluatedExpression::Action(action) => action.span(),
			EvaluatedExpression::FunctionCall(action_call) => action_call.span(),
			EvaluatedExpression::Parenthesized(parenthesized) => parenthesized.span(),
		}
	}
}
