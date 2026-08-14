use std::{
	borrow::Cow,
	ops::Deref,
	path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
	/// The zero-indexed start byte index of the span.
	start: usize,
	/// The length of the span.
	length: usize,

	file: Option<PathBuf>,
}

impl Span {
	pub fn from_start_and_length<P: AsRef<Path>>(file: P, start: usize, length: usize) -> Span {
		Span {
			start,
			length,
			file: Some(file.as_ref().to_path_buf()),
		}
	}

	pub fn eof<P: AsRef<Path>>(file: P) -> Span {
		let file = file.as_ref();
		let bytes = std::fs::read(file).unwrap().len();
		Span::from_start_and_length(file, bytes - 1, 1)
	}

	pub const fn none() -> Span {
		Span { start: 0, length: 1, file: None }
	}

	pub fn cover(first: &Span, second: &Span) -> Span {
		Span {
			file: first.file.clone(),
			start: first.start,
			length: (second.start + second.length).abs_diff(first.start),
		}
	}

	pub fn range<P: AsRef<Path>>(file: P, start: usize, end: usize) -> Span {
		Span {
			file: Some(file.as_ref().to_path_buf()),
			start,
			length: end - start,
		}
	}

	pub fn or<'other, A: Into<Cow<'other, Span>>>(&self, other: A) -> Span {
		if self.file.is_none() { other.into().into_owned() } else { self.clone() }
	}

	pub fn contains(&self, position: usize) -> bool {
		(self.start..self.start + self.length).contains(&position)
	}
}

pub trait Spanned {
	/// Returns the section of the source code that this expression spans. This is used by the compiler to print information about
	/// errors that occur, such as while line and column the error occurred on.
	///
	/// # Returns
	///
	/// The section of the program's source code that this expression spans.
	fn span(&self) -> impl SpanLike;
}

impl From<Span> for Cow<'static, Span> {
	fn from(val: Span) -> Self {
		Cow::Owned(val)
	}
}

impl<'span> From<&'span Span> for Cow<'span, Span> {
	fn from(val: &'span Span) -> Self {
		Cow::Borrowed(val)
	}
}

impl AsRef<Span> for Span {
	fn as_ref(&self) -> &Span {
		self
	}
}

impl Spanned for Span {
	fn span(&self) -> impl SpanLike<'_> {
		self
	}
}

impl Spanned for &Span {
	fn span(&self) -> impl SpanLike<'_> {
		*self
	}
}

#[derive(Debug, Clone)]
pub struct CowSpan<'span> {
	inner: Cow<'span, Span>,
}

impl AsRef<Span> for CowSpan<'_> {
	fn as_ref(&self) -> &Span {
		match &self.inner {
			Cow::Borrowed(span) => span,
			Cow::Owned(span) => span,
		}
	}
}

pub trait SpanLike<'span>: Sized + AsRef<Span> {
	fn into_cow_span(self) -> CowSpan<'span>;

	fn into_owned(self) -> Span {
		self.into_cow_span().inner.into_owned()
	}

	fn file(&self) -> &Path {
		self.as_ref().file.as_ref().unwrap()
	}

	fn length(&self) -> usize {
		self.as_ref().length
	}

	fn start(&self) -> usize {
		self.as_ref().start
	}

	fn end(&self) -> usize {
		let span = self.as_ref();
		span.start + span.length
	}

	#[allow(single_use_lifetimes, reason = "false positive; impl Trait<'_> is unstable")]
	fn to<'spn>(&self, other: impl SpanLike<'spn>) -> Span {
		Span::cover(self.as_ref(), other.as_ref())
	}

	fn owned(&self) -> Span {
		self.as_ref().to_owned()
	}

	/// # Returns
	///
	/// The `(line, column)` of the start of this span, or `None` if it's out of range for the
	/// given text.
	fn start_line_column(&self) -> Option<(usize, usize)> {
		let text = std::fs::read_to_string(self.file()).expect("Attempted to read Span::none()");

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

	fn end_line_column(&self) -> Option<(usize, usize)> {
		let text = std::fs::read_to_string(self.file()).expect("Attempted to read Span::none()");

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

impl<'span> SpanLike<'span> for Span {
	fn into_cow_span(self) -> CowSpan<'span> {
		CowSpan { inner: Cow::Owned(self) }
	}
}

impl<'span> SpanLike<'span> for &'span Span {
	fn into_cow_span(self) -> CowSpan<'span> {
		CowSpan { inner: Cow::Borrowed(self) }
	}
}

impl<'span> SpanLike<'span> for CowSpan<'span> {
	fn into_cow_span(self) -> CowSpan<'span> {
		self
	}
}

impl<'span> Deref for CowSpan<'span> {
	type Target = Cow<'span, Span>;

	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
