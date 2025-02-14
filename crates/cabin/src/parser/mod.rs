use std::collections::VecDeque;

use crate::{
	ast::statements::Statement,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
	Context,
	Span,
};

#[derive(Clone, Debug, thiserror::Error)]
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
pub trait TokenQueueFunctionality {
	/// Removes and returns the next token's value in the queue if the token matches the given token type. If it
	/// does not (or the token stream is empty), an error is returned.
	///
	/// # Parameters
	/// - `token_type` - The type of token to pop.
	///
	/// # Returns
	/// A `Result` containing either the value of the popped token or an `Error`.
	fn pop(&mut self, token_type: TokenType) -> Result<Token, Diagnostic>;

	/// Removes and returns the next token's type in the queue if the token matches the given token type. If it
	/// does not (or the token stream is empty), an error is returned.
	///
	/// # Parameters
	/// - `token_type` - The type of token to pop.
	///
	/// # Returns
	/// A `Result` containing either the type of the popped token or an `Error`.
	fn pop_type(&mut self, token_type: TokenType) -> Result<TokenType, Diagnostic>;

	/// Returns a reference to the next token in the queue without removing it. If the queue is empty, `None`
	/// is returned.
	///
	/// # Returns
	/// A reference to the next token in the queue or `None` if the queue is empty.
	fn peek(&self) -> Result<&str, Diagnostic>;

	fn peek_type(&self) -> Result<TokenType, Diagnostic>;

	fn peek_type2(&self) -> Result<TokenType, Diagnostic>;

	/// Returns whether the next token in the queue matches the given token type.
	fn next_is(&self, token_type: TokenType) -> bool {
		self.peek_type().map_or(false, |token| token == token_type)
	}

	/// Returns whether the next next token in the queue matches the given token type.
	fn next_next_is(&self, token_type: TokenType) -> bool {
		self.peek_type2().map_or(false, |token| token == token_type)
	}

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
	fn peek(&self) -> Result<&str, Diagnostic> {
		let mut index = 0;
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next.token_type.is_whitespace() {
			index += 1;
			next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}
		Ok(&next.value)
	}

	fn peek_type(&self) -> Result<TokenType, Diagnostic> {
		let mut index = 0;
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next.token_type.is_whitespace() {
			index += 1;
			next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}
		Ok(next.token_type)
	}

	fn peek_type2(&self) -> Result<TokenType, Diagnostic> {
		let mut index = 0;

		// The one time I'd enjoy a do-while loop
		let mut next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		index += 1;
		while next.token_type.is_whitespace() {
			next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
			index += 1;
		}

		let mut next_next = self.get(index).ok_or_else(|| Diagnostic {
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
		})?;
		while next_next.token_type.is_whitespace() {
			index += 1;
			next_next = self.get(index).ok_or_else(|| Diagnostic {
				span: Span::unknown(),
				info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedGenericEOF)),
			})?;
		}

		Ok(next_next.token_type)
	}

	fn is_all_whitespace(&self) -> bool {
		self.iter().all(|token| token.token_type.is_whitespace())
	}

	fn pop(&mut self, token_type: TokenType) -> Result<Token, Diagnostic> {
		let mut maybe_whitespace = TokenType::Whitespace;
		while maybe_whitespace.is_whitespace() {
			if let Some(token) = self.pop_front() {
				maybe_whitespace = token.token_type;

				if token.token_type == token_type {
					return Ok(token);
				}

				if !maybe_whitespace.is_whitespace() {
					return Err(Diagnostic {
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
			span: Span::unknown(),
			info: DiagnosticInfo::Error(crate::Error::Parse(ParseError::UnexpectedEOF { expected: token_type })),
		});
	}

	fn pop_type(&mut self, token_type: TokenType) -> Result<TokenType, Diagnostic> {
		let mut maybe_whitespace = TokenType::Whitespace;
		while maybe_whitespace.is_whitespace() {
			if let Some(token) = self.pop_front() {
				maybe_whitespace = token.token_type;

				if token.token_type == token_type {
					return Ok(token.token_type);
				}

				if !maybe_whitespace.is_whitespace() {
					return Err(Diagnostic {
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

pub trait TryParse {
	type Output;

	fn try_parse(tokens: &mut TokenQueue, context: &mut Context) -> Result<Self::Output, Diagnostic>;
}

pub trait Parse {
	type Output;

	fn parse(tokens: &mut TokenQueue, context: &mut Context) -> Self::Output;
}

pub type TokenQueue = VecDeque<Token>;
