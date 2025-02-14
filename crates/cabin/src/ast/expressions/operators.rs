use std::collections::VecDeque;

use super::extend::Extend;
use crate::{
	api::context::Context,
	ast::{
		expressions::{
			block::Block,
			either::Either,
			foreach::ForEachLoop,
			function_call::{FunctionCall, PostfixOperators},
			function_declaration::FunctionDeclaration,
			group::GroupDeclaration,
			if_expression::IfExpression,
			name::Name,
			object::ObjectConstructor,
			oneof::OneOf,
			run::RunExpression,
			Expression,
		},
		sugar::{list::List, string::CabinString},
	},
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{Parse as _, ParseError, TokenQueueFunctionality as _, TryParse},
	Error,
};

/// A binary operation. More specifically, this represents not one operation, but a group of operations that share the same precedence.
/// For example, the `+` and `-` operators share the same precedence, so they are grouped together in the `ADDITIVE` constant.
///
/// # Parameters
/// `<'this>` - The lifetime of this operation, to ensure that the contained reference to the precedent operation lives at least that long.
pub struct BinaryOperation {
	/// The operation that has the next highest precedence, or `None` if this operation has the highest precedence.
	precedent: Option<&'static BinaryOperation>,
	/// The token types that represent this operation, used to parse a binary expression.
	token_types: &'static [TokenType],
}

impl BinaryOperation {
	/// Parses the precedent operation of this one if it exists, otherwise, parses a function call (which has higher precedence than any binary operation)
	///
	/// # Parameters
	/// - `tokens` - The token stream to parse
	/// - `current_scope` - The current scope
	/// - `debug_info` - The debug information
	fn parse_precedent(&self, tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Expression, Diagnostic> {
		if let Some(precedent) = self.precedent {
			parse_binary_expression(precedent, tokens, context)
		} else {
			PostfixOperators::try_parse(tokens, context)
		}
	}
}

/// A binary expression node in the abstract syntax tree. This represents an operation that takes two operands in infix notation.
#[derive(Clone, Debug)]
pub struct BinaryExpression;

fn parse_binary_expression(operation: &BinaryOperation, tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Expression, Diagnostic> {
	let mut expression = operation.parse_precedent(tokens, context)?;

	while tokens.next_is_one_of(operation.token_types) {
		let operator_token = tokens.pop(tokens.peek_type()?)?;
		let right = operation.parse_precedent(tokens, context)?;
		expression = Expression::FunctionCall(FunctionCall::from_binary_operation(context, expression, right, operator_token)?);
	}

	Ok(expression)
}

impl TryParse for BinaryExpression {
	type Output = Expression;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		parse_binary_expression(&COMBINATOR, tokens, context)
	}
}

pub struct PrimaryExpression;

impl TryParse for PrimaryExpression {
	type Output = Expression;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		Ok(match tokens.peek_type()? {
			TokenType::LeftParenthesis => {
				let _ = tokens.pop(TokenType::LeftParenthesis).unwrap_or_else(|_| unreachable!());
				let expression = Expression::parse(tokens, context);
				let _ = tokens.pop(TokenType::RightParenthesis)?;
				expression
				// TODO: this needs to be its own expression type for transpilation
			},

			// Parse function declaration expression
			TokenType::KeywordAction => Expression::Pointer(FunctionDeclaration::try_parse(tokens, context)?),

			// Parse block expression
			TokenType::LeftBrace => Expression::Block(Block::try_parse(tokens, context)?),

			// Parse variable name expression
			TokenType::Identifier => Expression::Name(Name::try_parse(tokens, context)?),

			// Parse object constructor
			TokenType::KeywordNew => Expression::ObjectConstructor(ObjectConstructor::try_parse(tokens, context)?),

			// Parse group declaration expression
			TokenType::KeywordGroup => Expression::Pointer(GroupDeclaration::try_parse(tokens, context)?),

			// Parse one-of declaration expression
			TokenType::KeywordOneOf => Expression::Pointer(OneOf::try_parse(tokens, context)?),

			TokenType::KeywordEither => Expression::Pointer(Either::try_parse(tokens, context)?),
			TokenType::KeywordIf => Expression::If(IfExpression::try_parse(tokens, context)?),
			TokenType::KeywordForEach => Expression::ForEachLoop(ForEachLoop::try_parse(tokens, context)?),
			TokenType::KeywordExtend => Expression::Pointer(Extend::try_parse(tokens, context)?),

			// Parse run expression
			TokenType::KeywordRuntime => Expression::Run(RunExpression::try_parse(tokens, context)?),

			// Syntactic sugar: These below handle cases where syntactic sugar exists for initializing objects of certain types, such as
			// strings, numbers, lists, etc.:

			// Parse list literal into a list object
			TokenType::LeftBracket => List::try_parse(tokens, context)?,

			// Parse string literal into a string object
			TokenType::String => CabinString::try_parse(tokens, context)?,

			// Parse number literal into a number object
			TokenType::Number => {
				let number_token = tokens.pop(TokenType::Number).unwrap();
				Expression::ObjectConstructor(ObjectConstructor::number(number_token.value.parse().unwrap(), number_token.span, context))
			},

			// bad :<
			token_type => {
				return Err(Diagnostic {
					span: tokens.current_position().unwrap(),
					info: DiagnosticInfo::Error(Error::Parse(ParseError::UnexpectedTokenExpected {
						expected: "primary expression",
						actual: token_type,
					})),
				})
			},
		})
	}
}

static PIPE: BinaryOperation = BinaryOperation {
	precedent: None,
	token_types: &[TokenType::RightArrow],
};

// TODO: make this right-associative
/// The exponentiation operation, which has the highest precedence. This covers the `^` operator.
static EXPONENTIATION: BinaryOperation = BinaryOperation {
	precedent: Some(&PIPE),
	token_types: &[TokenType::Caret],
};

/// The multiplicative operations, which have the second highest precedence. This covers the `*` and `/` operators.
static MULTIPLICATIVE: BinaryOperation = BinaryOperation {
	precedent: Some(&EXPONENTIATION),
	token_types: &[TokenType::Asterisk, TokenType::ForwardSlash],
};

/// The additive operations, which have the third precedence. This covers the `+` and `-` operators.
static ADDITIVE: BinaryOperation = BinaryOperation {
	precedent: Some(&MULTIPLICATIVE),
	token_types: &[TokenType::Plus, TokenType::Minus],
};

/// The comparison operations, such as "==", "<=", etc.
static COMPARISON: BinaryOperation = BinaryOperation {
	precedent: Some(&ADDITIVE),
	token_types: &[TokenType::DoubleEquals, TokenType::LessThan, TokenType::GreaterThan],
};

static COMBINATOR: BinaryOperation = BinaryOperation {
	precedent: Some(&COMPARISON),
	token_types: &[TokenType::KeywordAnd, TokenType::KeywordOr],
};
