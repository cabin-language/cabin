use std::collections::VecDeque;

use crate::{
	Context,
	Span,
	diagnostics::{Diagnostic, DiagnosticInfo},
	lexer::{Token, TokenType},
};

#[derive(Clone, Debug, thiserror::Error, Hash, PartialEq, Eq)]
pub enum ParseError {}

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
}

impl TokenQueueFunctionality for TokenQueue {
	fn peek_type(&self, context: &Context) -> Result<TokenType, Diagnostic> {
		Ok(self
			.front()
			.ok_or_else(|| Diagnostic {
				file: context.file.clone(),
				span: Span::none(),
				info: DiagnosticInfo::UnexpectedGenericEOF,
			})?
			.token_type)
	}

	fn next_is(&self, token_type: TokenType) -> bool {
		self.front().is_some_and(|front| front.token_type == token_type)
	}

	fn peek_type2(&self, context: &Context) -> Result<TokenType, Diagnostic> {
		Ok(self
			.get(1)
			.ok_or_else(|| Diagnostic {
				file: context.file.clone(),
				span: Span::none(),
				info: DiagnosticInfo::UnexpectedGenericEOF,
			})?
			.token_type)
	}

	fn pop(&mut self, token_type: TokenType, context: &Context) -> Result<Token, Diagnostic> {
		if let Some(front) = self.front() {
			if front.token_type == token_type {
				return Ok(self.pop_front().unwrap());
			}

			return Err(Diagnostic {
				span: front.span,
				info: DiagnosticInfo::UnexpectedToken {
					expected: token_type,
					actual: front.token_type,
				},
				file: context.file.clone(),
			});
		}

		Err(Diagnostic {
			file: context.file.clone(),
			span: Span::none(),
			info: DiagnosticInfo::UnexpectedEOF { expected: token_type },
		})
	}

	fn current_position(&self) -> Option<Span> {
		self.front().map(|front| front.span)
	}
}

pub enum ListType {
	AngleBracketed,
	Braced,
	Bracketed,
	Parenthesized,
	Tag,
}

impl ListType {
	pub const fn opening(&self) -> TokenType {
		match self {
			Self::AngleBracketed => TokenType::LeftAngleBracket,
			Self::Braced => TokenType::LeftBrace,
			Self::Bracketed => TokenType::LeftBracket,
			Self::Parenthesized => TokenType::LeftParenthesis,
			Self::Tag => TokenType::TagOpening,
		}
	}

	pub const fn closing(&self) -> TokenType {
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
