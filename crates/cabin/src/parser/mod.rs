use std::collections::VecDeque;

use crate::{
	ast::statements::Statement,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	Context,
	Span,
};

#[derive(Clone, Debug, thiserror::Error, Hash, PartialEq, Eq)]
pub enum ParseError {
	#[error("Unexpected token: Expected {expected} but found {actual}")]
	UnexpectedToken { expected: TokenType, actual: TokenType },

	#[error("Unexpected token: Expected {expected} but found {actual}")]
	UnexpectedTokenExpected { expected: &'static str, actual: TokenType },

	#[error("Unexpected end of file: Expected {expected} but found end of file")]
	UnexpectedEOF { expected: TokenType },

	#[error("Unexpected end of file: Expected more tokens")]
	UnexpectedGenericEOF,

	#[error("The variable \"{name}\" was declared twice")]
	DuplicateVariableDeclaration { name: String },

	#[error("Only declarations are allowed at the top level of a module")]
	InvalidTopLevelStatement { statement: Statement },

	#[error("Invalid formatted string: {0}")]
	InvalidFormatString(String),

	#[error("Duplicate field \"{0}\"")]
	DuplicateField(String),
}

/// A trait for treating a collection of tokens as a queue of tokens that can be parsed. This is
/// traditionally implemented for `VecDeque<Token>`.
pub(crate) trait TokenQueueFunctionality {
	/// Removes and returns the next token's value in the queue if the token matches the given token type. If it
	/// does not (or the token stream is empty), an error is returned.
	///
	/// # Parameters
	/// - `token_type` - The type of token to pop.
	///
	/// # Returns
	/// A `Result` containing either the value of the popped token or an `Error`.
	fn pop(&mut self, token_type: TokenType, context: &Context) -> Result<Token, Diagnostic>;

	fn peek_type(&self, context: &Context) -> Result<TokenType, Diagnostic>;

	fn peek_type2(&self, context: &Context) -> Result<TokenType, Diagnostic>;

	/// Returns whether the next token in the queue matches the given token type.
	fn next_is(&self, token_type: TokenType) -> bool;

	/// Returns whether the next token in the queue matches one of the given token types.
	///
	/// # Parameters
	/// - `token_types` - The token types to check against.
	///
	/// # Returns
	/// Whether the next token in the queue matches one of the given token types.
	fn next_is_one_of(&self, token_types: &[TokenType]) -> bool {
		token_types.iter().any(|token_type| self.next_is(token_type.to_owned()))
	}

	fn current_position(&self) -> Option<Span>;

	fn is_all_whitespace(&self) -> bool;
}

impl TokenQueueFunctionality for TokenQueue {
	fn peek_type(&self, context: &Context) -> Result<TokenType, Diagnostic> {
		let mut index = 0;
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			file: context.file.clone(),
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next.token_type.is_whitespace() {
			index += 1;
			next = self.get(index).ok_or_else(|| Diagnostic {
				file: context.file.clone(),
				span: Span::unknown(),
				info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}
		Ok(next.token_type)
	}

	fn next_is(&self, token_type: TokenType) -> bool {
		let mut index = 0;
		let Some(mut next) = self.get(index) else { return false; };
		while next.token_type.is_whitespace() {
			if token_type == TokenType::Comment && next.token_type == TokenType::Comment {
				return true;
			}
			index += 1;
			if let Some(token) = self.get(index) {
				next = token;
			} else {
				return false;
			}
		}
		next.token_type == token_type
	}

	fn peek_type2(&self, context: &Context) -> Result<TokenType, Diagnostic> {
		let mut index = 0;

		// The one time I'd enjoy a do-while loop
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			file: context.file.clone(),
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		index += 1;
		while next.token_type.is_whitespace() {
			next = self.get(index).ok_or_else(|| Diagnostic {
				file: context.file.clone(),
				span: Span::unknown(),
				info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
			index += 1;
		}

		let mut next_next = self.get(index).ok_or_else(|| Diagnostic {
			file: context.file.clone(),
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next_next.token_type.is_whitespace() {
			index += 1;
			next_next = self.get(index).ok_or_else(|| Diagnostic {
				file: context.file.clone(),
				span: Span::unknown(),
				info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}

		Ok(next_next.token_type)
	}

	fn is_all_whitespace(&self) -> bool {
		self.iter().all(|token| token.token_type.is_whitespace())
	}

	fn pop(&mut self, token_type: TokenType, context: &Context) -> Result<Token, Diagnostic> {
		let mut maybe_whitespace = TokenType::Whitespace;
		while maybe_whitespace.is_whitespace() {
			if let Some(token) = self.pop_front() {
				maybe_whitespace = token.token_type;

				if token.token_type == token_type {
					return Ok(token);
				}

				if !maybe_whitespace.is_whitespace() {
					return Err(Diagnostic {
						file: context.file.clone(),
						span: token.span,
						info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedToken {
							expected: token_type,
							actual: token.token_type,
						})),
					});
				}
			}
		}

		return Err(Diagnostic {
			file: context.file.clone(),
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedEOF { expected: token_type })),
		});
	}

	fn current_position(&self) -> Option<Span> {
		self.front().map(|front| front.span)
	}
}

pub(crate) enum ListType {
	AngleBracketed,
	Braced,
	Bracketed,
	Parenthesized,
	Tag,
}

impl ListType {
	pub(crate) const fn opening(&self) -> TokenType {
		match self {
			Self::AngleBracketed => TokenType::LeftAngleBracket,
			Self::Braced => TokenType::LeftBrace,
			Self::Bracketed => TokenType::LeftBracket,
			Self::Parenthesized => TokenType::LeftParenthesis,
			Self::Tag => TokenType::TagOpening,
		}
	}

	pub(crate) const fn closing(&self) -> TokenType {
		match self {
			Self::AngleBracketed => TokenType::RightAngleBracket,
			Self::Braced => TokenType::RightBrace,
			Self::Parenthesized => TokenType::RightParenthesis,
			Self::Bracketed | Self::Tag => TokenType::RightBracket,
		}
	}
}

pub(crate) trait TryParse {
	type Output;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic>;
}

pub(crate) trait Parse {
	type Output;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output;
}

pub(crate) type TokenQueue = VecDeque<Token>;
