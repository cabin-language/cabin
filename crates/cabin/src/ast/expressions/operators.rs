use std::collections::VecDeque;

use super::new_literal::{EvaluatedLiteral, UnevaluatedLiteral};
use crate::{
	api::context::Context,
	ast::{
		expressions::{
			block::Block,
			either::Either,
			extend::Extend,
			foreach::ForEachLoop,
			function_call::{FunctionCall, PostfixOperators},
			function_declaration::FunctionDeclaration,
			group::GroupDeclaration,
			if_expression::IfExpression,
			name::Name,
			object::ObjectConstructor,
			run::RunExpression,
			Expression,
		},
		sugar::{list::List, string::CabinString},
	},
	comptime::memory::ExpressionPointer,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	parser::{Parse as _, ParseError, TokenQueueFunctionality as _, TryParse},
};

/// A binary operation. More specifically, this represents not one operation, but a group of operations that share the same precedence.
/// For example, the `+` and `-` operators share the same precedence, so they are grouped together in the `ADDITIVE` constant.
///
/// # Parameters
/// `<'this>` - The lifetime of this operation, to ensure that the contained reference to the precedent operation lives at least that long.
pub(crate) struct BinaryOperation {
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
	fn parse_precedent(&self, tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<ExpressionPointer, Diagnostic> {
		if let Some(precedent) = self.precedent {
			parse_binary_expression(precedent, tokens, context)
		} else {
			PostfixOperators::try_parse(tokens, context)
		}
	}
}

/// A binary expression node in the abstract syntax tree. This represents an operation that takes two operands in infix notation.
#[derive(Clone, Debug)]
pub(crate) struct BinaryExpression;

fn parse_binary_expression(operation: &BinaryOperation, tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<ExpressionPointer, Diagnostic> {
	let mut expression = operation.parse_precedent(tokens, context)?;

	while tokens.next_is_one_of(operation.token_types) {
		let operator_token = tokens.pop(tokens.peek_type(context)?, context)?;
		let right = operation.parse_precedent(tokens, context)?;
		expression = Expression::FunctionCall(FunctionCall::from_binary_operation(context, expression, right, operator_token)?).store_in_memory(context);
	}

	Ok(expression)
}

impl TryParse for BinaryExpression {
	type Output = ExpressionPointer;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		parse_binary_expression(&COMBINATOR, tokens, context)
	}
}

pub struct PrimaryExpression;

impl TryParse for PrimaryExpression {
	type Output = ExpressionPointer;

	fn try_parse(tokens: &mut VecDeque<Token>, context: &mut Context) -> Result<Self::Output, Diagnostic> {
		Ok(match tokens.peek_type(context)? {
			TokenType::LeftParenthesis => {
				let _ = tokens.pop(TokenType::LeftParenthesis, context).unwrap_or_else(|_| unreachable!());
				let expression = Expression::parse(tokens, context);
				let _ = tokens.pop(TokenType::RightParenthesis, context)?;
				expression
				// TODO: this needs to be its own expression type for transpilation/formatting
			},

			TokenType::KeywordAction => Expression::Literal(UnevaluatedLiteral::FunctionDeclaration(FunctionDeclaration::try_parse(tokens, context)?)).store_in_memory(context),
			TokenType::LeftBrace => Expression::Block(Block::try_parse(tokens, context)?).store_in_memory(context),
			TokenType::Identifier => Expression::Name(Name::try_parse(tokens, context)?).store_in_memory(context),
			TokenType::KeywordNew => Expression::ObjectConstructor(ObjectConstructor::try_parse(tokens, context)?).store_in_memory(context),
			TokenType::KeywordGroup => Expression::Literal(UnevaluatedLiteral::Group(GroupDeclaration::try_parse(tokens, context)?)).store_in_memory(context),
			TokenType::KeywordEither => Expression::Literal(UnevaluatedLiteral::Either(Either::try_parse(tokens, context)?)).store_in_memory(context),
			TokenType::KeywordIf => Expression::If(IfExpression::try_parse(tokens, context)?).store_in_memory(context),
			TokenType::KeywordForEach => Expression::ForEachLoop(ForEachLoop::try_parse(tokens, context)?).store_in_memory(context),
			TokenType::KeywordExtend => Expression::Literal(UnevaluatedLiteral::Extend(Extend::try_parse(tokens, context)?)).store_in_memory(context),

			// Syntactic sugar: These below handle cases where syntactic sugar exists for initializing objects of certain types, such as
			// strings, numbers, lists, etc.:
			TokenType::LeftBracket => Expression::List(List::try_parse(tokens, context)?).store_in_memory(context),
			TokenType::String => CabinString::try_parse(tokens, context)?,
			TokenType::Number => {
				let number_token = tokens.pop(TokenType::Number, context).unwrap();
				Expression::EvaluatedLiteral(EvaluatedLiteral::Number(number_token.value.parse().unwrap())).store_in_memory(context)
			},

			// bad :<
			token_type => {
				return Err(Diagnostic {
					file: context.file.clone(),
					span: tokens.current_position().unwrap(),
					info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedTokenExpected {
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
