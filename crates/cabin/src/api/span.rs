use super::io::Io;
use crate::Context;

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct Span {
	/// The zero-indexed start byte index of the span.
	pub start: usize,
	/// The length of the span.
	pub length: usize,
}

impl Span {
	pub const fn new(start: usize, length: usize) -> Span {
		Span { start, length }
	}

	pub const fn unknown() -> Span {
		Span { start: 0, length: 1 }
	}

	pub const fn cover(first: Span, second: Span) -> Span {
		Span {
			start: first.start,
			length: (second.start + second.length).abs_diff(first.start),
		}
	}

	pub const fn to(self, other: Span) -> Span {
		Span::cover(self, other)
	}

	pub fn or(self, other: Span) -> Span {
		if self == Span::unknown() {
			other
		} else {
			self
		}
	}

	pub fn contains(&self, position: usize) -> bool {
		(self.start..self.start + self.length).contains(&position)
	}

	pub const fn start(&self) -> usize {
		self.start
	}

	pub const fn end(&self) -> usize {
		self.start + self.length
	}

	pub const fn length(&self) -> usize {
		self.length
	}

	/// Converts the start value of this span from a byte position to a line-column position in the
	/// given text. If this span's start byte index greater than the text's length, `None` is
	/// returned.
	///
	/// # Parameters
	///
	/// - `text` - The text to convert from byte position to line-column
	///
	/// # Returns
	///
	/// The `(line, column)` of the start of this span, or `None` if it's out of range for the
	/// given text.
	pub fn start_line_column(&self, text: &str) -> Option<(usize, usize)> {
		let mut line = 0;
		let mut column = 0;
		for (position, character) in text.chars().enumerate() {
			if position == self.start() {
				return Some((line, column));
			}

			column += 1;
			if character == '\n' {
				line += 1;
				column = 0;
			}
		}

		None
	}

	pub fn end_line_column(&self, text: &str) -> Option<(usize, usize)> {
		let mut line = 0;
		let mut column = 0;
		for (position, character) in text.chars().enumerate() {
			if position == self.end() {
				return Some((line, column));
			}

			column += 1;
			if character == '\n' {
				line += 1;
				column = 0;
			}
		}

		None
	}
}

pub trait Spanned {
	/// Returns the section of the source code that this expression spans. This is used by the compiler to print information about
	/// errors that occur, such as while line and column the error occurred on.
	///
	/// # Returns
	///
	/// The second of the program's source code that this expression spans.
	fn span<System: Io>(&self, context: &Context<System>) -> Span;
}
