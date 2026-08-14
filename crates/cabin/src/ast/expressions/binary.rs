use std::collections::VecDeque;

use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::{
		UnevaluatedExpression,
		action::Action,
		block::Block,
		either::Either,
		extend::Extend,
		foreach::ForLoop,
		function_call::{FunctionCall, PostfixExpression},
		group::Group,
		identifier::Identifier,
		if_expression::IfExpression,
		list::List,
		new::NewExpression,
		number::Number,
		parenthesized::ParenthesizedExpression,
		string::Text,
	},
	comptime::{
		Evaluate,
		storage::{EvaluatedRef, ExpressionRef},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType,
	transpiler::ToCabin,
};

#[derive(Clone, Debug, Getters)]
pub struct BinaryExpression {
	left: ExpressionRef,
	operation: BinaryOperation,
	right: ExpressionRef,
	tokens: BinaryExpressionTokens,
}

#[derive(Clone, Debug, Getters)]
pub struct BinaryExpressionTokens {
	operator: Token,
}

impl TryParse for BinaryExpression {
	type Output = ExpressionRef;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		BinaryOperation::Or.parse(tokens, context)
	}
}

impl Evaluate for BinaryExpression {
	type Output = EvaluatedRef;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating binary expression");
		let operation = *self.operation();
		FunctionCall::from_binary_expression(self.left, operation, self.right, self.tokens.operator, context).evaluate(context)
	}
}

impl Spanned for BinaryExpression {
	fn span(&self) -> impl SpanLike<'_> {
		self.left.span().to(self.right.span())
	}
}

impl ToCabin for BinaryExpression {
	fn to_cabin(&self) -> String {
		format!("{} {} {}", self.left.to_cabin(), self.operation.to_cabin(), self.right.to_cabin())
	}
}

impl BinaryExpression {
	pub const fn synthetic(left: ExpressionRef, operator: Token, right: ExpressionRef) -> BinaryExpression {
		BinaryExpression {
			left,
			right,
			operation: BinaryOperation::from_token_type(operator.token_type).unwrap(),
			tokens: BinaryExpressionTokens { operator },
		}
	}
}

pub struct PrimaryExpression;

impl TryParse for PrimaryExpression {
	type Output = ExpressionRef;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		Ok(match tokens.peek_type(context)? {
			TokenType::LeftParenthesis => Box::new(ParenthesizedExpression::try_parse(tokens, context)?).into(),
			TokenType::KeywordAction => Box::new(Action::try_parse(tokens, context)?).into(),
			TokenType::LeftBrace => Block::try_parse(tokens, context, ScopeType::Block)?.into(),
			TokenType::Identifier => Identifier::try_parse(tokens, context)?.into(),
			TokenType::KeywordNew => NewExpression::try_parse(tokens, context)?.into(),
			TokenType::KeywordGroup => Group::try_parse(tokens, context)?.into(),
			TokenType::KeywordEither => Either::try_parse(tokens, context)?.into(),
			TokenType::KeywordIf => Box::new(IfExpression::try_parse(tokens, context)?).into(),
			TokenType::KeywordForEach => Box::new(ForLoop::try_parse(tokens, context)?).into(),
			TokenType::KeywordExtend => Box::new(Extend::try_parse(tokens, context)?).into(),
			TokenType::String => Text::try_parse(tokens, context)?.into(),
			TokenType::Number => Number::try_parse(tokens, context)?.into(),
			TokenType::LeftBracket => List::try_parse(tokens, context)?.into(),

			// bad :<
			token_type => {
				return Err(Diagnostic::at(tokens.peek(context).unwrap(), DiagnosticInfo::UnexpectedTokenExpected {
					expected: "primary expression",
					actual: token_type,
				}));
			},
		})
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum BinaryOperation {
	Exponent,

	Multiply,
	Divide,

	Add,
	Subtract,

	LessThan,
	GreaterThan,
	Equal,
	NotEqual,
	LessThanOrEqual,
	GreaterThanOrEqual,

	Or,
	And,
}

impl BinaryOperation {
	fn precedent(self) -> Option<BinaryOperation> {
		match self {
			Self::Exponent => None,
			Self::Multiply => Some(Self::Exponent),
			Self::Add => Some(Self::Multiply),
			Self::LessThan => Some(Self::Add),
			Self::Or => Some(Self::LessThan),
			_ => unreachable!(),
		}
	}

	fn token_types(self) -> &'static [TokenType] {
		match self {
			Self::Exponent => &[TokenType::Caret],
			Self::Multiply => &[TokenType::Asterisk, TokenType::ForwardSlash, TokenType::KeywordMod],
			Self::Add => &[TokenType::Plus, TokenType::Minus],
			Self::LessThan => &[
				TokenType::LessThan,
				TokenType::GreaterThan,
				TokenType::DoubleEquals,
				TokenType::LessThanOrEqual,
				TokenType::GreaterThanOrEqual,
			],
			Self::Or => &[TokenType::KeywordOr, TokenType::KeywordAnd],
			_ => unreachable!(),
		}
	}

	fn parse_precedent(self, tokens: &mut TokenQueue, context: &mut Context) -> Result<ExpressionRef, Diagnostic> {
		if let Some(precedent) = self.precedent() {
			precedent.parse(tokens, context)
		} else {
			PostfixExpression::try_parse(tokens, context)
		}
	}

	fn parse(self, tokens: &mut TokenQueue, context: &mut Context) -> Result<ExpressionRef, Diagnostic> {
		let mut expression = self.parse_precedent(tokens, context)?;

		while tokens.next_is_one_of(self.token_types()) {
			let operator = tokens.pop(tokens.peek_type(context)?, context)?;
			let right = self.parse_precedent(tokens, context)?;
			expression = UnevaluatedExpression::Binary(Box::new(BinaryExpression {
				left: expression,
				operation: BinaryOperation::from_token_type(operator.token_type).unwrap(),
				right,
				tokens: BinaryExpressionTokens { operator },
			}));
		}

		Ok(expression)
	}

	const fn from_token_type(token_type: TokenType) -> Option<BinaryOperation> {
		match token_type {
			TokenType::Caret => Some(BinaryOperation::Exponent),
			TokenType::Asterisk => Some(BinaryOperation::Multiply),
			TokenType::ForwardSlash => Some(BinaryOperation::Divide),
			TokenType::Plus => Some(BinaryOperation::Add),
			TokenType::Minus => Some(BinaryOperation::Subtract),
			TokenType::LessThan => Some(BinaryOperation::LessThan),
			TokenType::LessThanOrEqual => Some(BinaryOperation::LessThanOrEqual),
			TokenType::GreaterThan => Some(BinaryOperation::GreaterThan),
			TokenType::GreaterThanOrEqual => Some(BinaryOperation::GreaterThanOrEqual),
			TokenType::DoubleEquals => Some(BinaryOperation::Equal),
			TokenType::KeywordOr => Some(BinaryOperation::Or),
			TokenType::KeywordAnd => Some(BinaryOperation::And),
			_ => None,
		}
	}

	pub const fn action_name(&self) -> &'static str {
		match self {
			BinaryOperation::Exponent => "to_the_power",
			BinaryOperation::Multiply => "times",
			BinaryOperation::Divide => "divided_by",
			BinaryOperation::Add => "plus",
			BinaryOperation::Subtract => "minus",
			BinaryOperation::LessThan => "is_less_than",
			BinaryOperation::GreaterThan => "is_greater_than",
			BinaryOperation::Equal => "equals",
			BinaryOperation::NotEqual => "not_equals",
			BinaryOperation::LessThanOrEqual => "less_than_or_equal_to",
			BinaryOperation::GreaterThanOrEqual => "greater_than_or_equal_to",
			BinaryOperation::Or => "or",
			BinaryOperation::And => "and",
		}
	}
}

impl ToCabin for BinaryOperation {
	fn to_cabin(&self) -> String {
		match self {
			BinaryOperation::Exponent => "^",
			BinaryOperation::Multiply => "*",
			BinaryOperation::Divide => "/",
			BinaryOperation::Add => "+",
			BinaryOperation::Subtract => "-",
			BinaryOperation::LessThan => "<",
			BinaryOperation::GreaterThan => ">",
			BinaryOperation::Equal => "=",
			BinaryOperation::NotEqual => "!=",
			BinaryOperation::LessThanOrEqual => "<=",
			BinaryOperation::GreaterThanOrEqual => ">=",
			BinaryOperation::Or => "or",
			BinaryOperation::And => "and",
		}
		.to_owned()
	}
}
