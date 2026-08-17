use std::collections::VecDeque;

use derive_getters::Getters;

use crate::{
	SpanLike,
	Spanned,
	api::context::Context,
	ast::expressions::{
		Expression,
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
		run::Runtime,
		string::Text,
	},
	comptime::{Evaluate, storage::Pointer},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	log,
	parser::{TokenQueue, TokenQueueFunctionality as _, TryParse},
	scope::ScopeType,
	transpiler::ToCabin,
};

#[derive(Clone, Debug, Getters)]
pub struct BinaryExpression {
	left: Pointer,
	operation: BinaryOperation,
	right: Pointer,
	tokens: BinaryExpressionTokens,
}

#[derive(Clone, Debug, Getters)]
pub struct BinaryExpressionTokens {
	operator: Token,
}

impl TryParse for BinaryExpression {
	type Output = Pointer;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		BinaryOperation::Or.parse(tokens, context)
	}
}

impl Evaluate for BinaryExpression {
	type Output = Pointer;

	fn evaluate(self, context: &mut Context) -> Self::Output {
		log!(context, "Evaluating binary expression");
		let operation = *self.operation();
		let span = self.span().owned();
		FunctionCall::from_binary_expression(self.left, operation, self.right, self.tokens.operator, context, span).evaluate(context)
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
	pub const fn synthetic(left: Pointer, operator: Token, right: Pointer) -> BinaryExpression {
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
	type Output = Pointer;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		Ok(match tokens.peek_type(context)? {
			TokenType::LeftParenthesis => Expression::from(ParenthesizedExpression::try_parse(tokens, context)?).store(context),
			TokenType::KeywordAction => Expression::from(Action::try_parse(tokens, context)?).store(context),
			TokenType::LeftBrace => Expression::from(Block::try_parse(tokens, context, ScopeType::Block)?).store(context),
			TokenType::Identifier => Expression::from(Identifier::try_parse(tokens, context)?).store(context),
			TokenType::KeywordNew => Expression::from(NewExpression::try_parse(tokens, context)?).store(context),
			TokenType::KeywordGroup => Expression::from(Group::try_parse(tokens, context)?).store(context),
			TokenType::KeywordEither => Expression::from(Either::try_parse(tokens, context)?).store(context),
			TokenType::KeywordIf => Expression::from(IfExpression::try_parse(tokens, context)?).store(context),
			TokenType::KeywordForEach => Expression::from(ForLoop::try_parse(tokens, context)?).store(context),
			TokenType::KeywordExtend => Expression::from(Extend::try_parse(tokens, context)?).store(context),
			TokenType::String => Expression::from(Text::try_parse(tokens, context)?).store(context),
			TokenType::Number => Expression::from(Number::try_parse(tokens, context)?).store(context),
			TokenType::LeftBracket => Expression::from(List::try_parse(tokens, context)?).store(context),

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

	fn parse_precedent(self, tokens: &mut TokenQueue, context: &mut Context) -> Result<Pointer, Diagnostic> {
		if let Some(precedent) = self.precedent() {
			precedent.parse(tokens, context)
		} else {
			PostfixExpression::try_parse(tokens, context)
		}
	}

	fn parse(self, tokens: &mut TokenQueue, context: &mut Context) -> Result<Pointer, Diagnostic> {
		let mut expression = self.parse_precedent(tokens, context)?;

		while tokens.next_is_one_of(self.token_types()) {
			let operator = tokens.pop(tokens.peek_type(context)?, context)?;
			let right = self.parse_precedent(tokens, context)?;
			expression = Expression::Binary(BinaryExpression {
				left: expression,
				operation: BinaryOperation::from_token_type(operator.token_type).unwrap(),
				right,
				tokens: BinaryExpressionTokens { operator },
			})
			.store(context);
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

impl Runtime for BinaryExpression {
	fn evaluate_subexpressions(self, context: &mut Context) -> Self {
		Self {
			left: self.left.evaluate(context),
			operation: self.operation,
			right: self.right.evaluate(context),
			tokens: self.tokens,
		}
	}
}
