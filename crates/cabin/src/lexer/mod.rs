use std::collections::VecDeque;

// Bring the `Casing` trait into scope, which allows us to use `.to_case()` on strings. This is used to convert `TokenType` enum variant names into Title Case,
// which is useful for outputting human-readable errors. We assign this to `as _` to indicate clearly that The trait is not referenced directly and only used
// to bring its methods into scope to be called.
use convert_case::Casing as _;
use regex_macro::{regex, Regex};
// Bring the `IntoEnumIterator` trait into scope, which allows us to use `::iter()` on enums that `#[derive(strum_macros::EnumIter)]`. Specifically here, this is used
// on `TokenType` to iterate over the values of the enum to test which one matches a specific string of Cabin code. We assign this to `as _` to indicate clearly that
// The trait is not referenced directly and only used to bring its methods into scope to be called.
use strum::IntoEnumIterator as _;

use crate::{
	api::context::context,
	cli::theme::{Style, Styled},
	ErrorInfo,
	Errors,
	PRELUDE,
};

/// A type of token in Cabin source code. The first step in Cabin compilation is tokenization, which is the process of splitting a raw String of source code into
/// "tokens" which each have a "type" representing the kind of token it is, and a "value" representing the string of source code that is associated with it. This
/// enum defines the different "types" of values. Tokens themselves are stored in a separate `Token` struct, which has a `token_type: TokenType` field. This *is*
/// against general Rust convention, which recommends instead making `value: String` a subtype of the `TokenType` enum. However, for this specific implementation and
/// use case, we want to be able to easily iterate over all types of tokens, which means we want each type to be a "concrete object" instead of an instantiable
/// function or type. Thus, we instead make `TokenType` a field on the `Token` struct.
///
/// # Conventions
///
/// These token types are named by what the token itself appears as, not the usage in the language. For example, the "`.`" token is not called "access" or something,
/// it is just called `Dot`. The names of the tokens should be written parser-agnostic, meaning they should have no "knowledge" of the actual use cases of the
/// token in the language. This helps make parser changes easier, as we can repurpose token types without having to rename them and without causing confusion
/// or ambiguity in what they refer to.
#[derive(strum_macros::EnumIter, PartialEq, Eq, Debug, Clone, Copy)]
pub enum TokenType {
	/// The "tag opening" token type. This marks the start of a list of tags on a variable declaration. Please note that this only notes the *start* of such
	/// a list, not the entire list. To be specific, this *only* matches the character sequence `#[`. All tokens after that sequenced are tokenized as normal, including
	/// the ending right bracket that tag lists conclude with. A returned token with this type will always have the value `#[`.
	TagOpening,

	/// The "asterisk" token type. This is used for parsing arithmetic multiplication expressions. Any token tokenized of this type will always have a single-character value,
	/// which is the "asterisk" character (*).
	Asterisk,

	/// The "caret" token type. This is used for parsing arithmetic exponential expressions. Any token tokenized of this type will always have a single-character value,
	/// which is the "caret" character (^).
	Caret,

	/// The "colon" token type. This is used for parsing explicit type tags on variable declarations. While Cabin does perform type inference and often allows type
	/// tags to be omitted, there are occasionally times where it's preferred or even necessary to insert a specific tag on a variable. Any token tokenized
	/// of this type will always have a single-character value, which is the "colon" character (:).
	Colon,

	/// The "comma" token type. This is used for things like function parameter separation, group field separation, etc. Any token tokenized of this type will always
	/// have a single-character value, which is the "comma" character (,).
	Comma,

	QuestionMark,
	ExclamationPoint,

	/// The double equals token type. This is used for comparisons in if statements, similar to other languages. Any token tokenized of this type will always
	/// have the value "=="
	DoubleEquals,

	/// The "dot" token type. This is used for things like function parameter separation, group field separation, etc. Any token tokenized of this type will always
	/// have a single-character value, which is the "comma" character (,).
	Dot,

	/// The "equal" token type. This is used for variable and field assignment. Any token tokenized of this type will always have a single-character value, which is
	/// the "equal" character (=).
	Equal,

	/// The "line comment" token type. This is used for comments the programmer wants to make about the code that doesn't actually affect the code at runtime.
	/// This represents the type of comment that continues until the end of the line. Currently, this is a double slash (//), but in the future will likely be
	/// changed to something more intuitive. When tokenizes, tokens of this type will return the entire comment, including the leading two slashes, but not the
	/// trailing newline or carriage return.
	///
	/// NOTE: This *must* be checked *before* the `ForwardSlash` token type. Failure to do so will start parsing a comment as two separate forward slash
	/// tokens, and then attempt to parse the comment as code. These token types are iterated with `strum::IntoEnumIterator`, which iterates over this enum
	/// in order. This means that this enum variant declaration *must* be placed *before* the `ForwardSlash` token type. Please be careful moving this variant
	/// or that one around!
	Comment,

	/// The forward slash token. This is used for arithmetic division expressions. Any token tokenized of this type will always have a single-character value, which is
	/// the "forward slash" character (/).
	ForwardSlash,

	/// The `otherwise` keyword token type. This is used similar to the "else" keyword in other languages: it runs a block of code if an `if` condition is false.
	///
	/// The design decision behind making this `otherwise` instead of `else` is that it reads more like natural English. It is less intuitive for experienced
	/// programmers, which may expect an `else` keyword to exist, but after learning it once it shouldn't be an issue.
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `otherwise` will be tokenized incorrectly as an
	/// identifier, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordOtherwise,

	RightArrow,

	KeywordExtend,
	KeywordAnd,
	KeywordOr,
	KeywordToBe,

	/// The `if` keyword token type. This is used similar to how it is in other languages: It runs a block of code if some condition is true.
	///
	/// Unlike many other keywords in Cabin, this is the same as it is in almost all other languages. Generally we swap out common keywords with things
	/// we find more intuitive or "common English" sounding, but `if` already is about as colloquial as it gets.
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `if` will be tokenized incorrectly as an
	/// identifier, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordIf,

	/// The `action` keyword token type. This is used to declare functions.
	///
	/// A token created with this type will always have the value "function".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `action` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordAction,

	/// The new keyword. This is used to instantiate a table.
	///
	/// A token created with this type will always have the value "new".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `new` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordNew,

	KeywordWith,

	/// The `group` keyword token type, which is used to declare a type of group of variables in the language, analogous to a `struct` in other languages.
	///
	/// This was named `group` instead of the far more common `struct` to be consistent with Cabin's readability and "common English" aesthetics. It's named
	/// exactly what it creates: A group of values.
	///
	/// A token created with this type will always have the value "group".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `group` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordGroup,

	/// The `run` keyword token type, which is used to run an expression at compile-time. In Cabin, all expressions are run at compile-time by default,
	/// but sometimes you want something specifically to be run at compile-time. This keyword forces an expression to be run at runtime. Note, however,
	/// that all sub-expressions should still be evaluated at compile-time. For example, the expression:
	///
	/// ```cabin
	/// run ((1 + 2) + (3 + 4))
	/// ```
	///
	/// at compile-time will convert to:
	///
	/// ```cabin
	/// run 3 + 7
	/// ```
	///
	/// To run it deeply nested, you'd have to do:
	///
	/// ```cabin
	/// run (run (1 + 2) + run (3 + 4))
	/// ```
	///
	/// A token created with this type will always have the value "run".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `run` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordRuntime,

	KeywordIs,

	KeywordMatch,

	/// The `either` keyword token type. This is used to create enumerations.
	///
	/// A token created with this type will always have the value "either".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `either` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordOneOf,

	/// The `let` keyword token type. This is used to declare a variable.
	///
	/// A token created with this type will always have the value "let".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `let` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordLet,

	/// The `foreach` keyword token type. This is used to iterate over a list.
	///
	/// A token created with this type will always have the value "foreach".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `foreach` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordForEach,

	/// The `in` keyword token type. This is used to iterate over a list.
	///
	/// A token created with this type will always have the value "in".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `in` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordIn,

	/// The `while` keyword token type. This is used to loop while some condition is true
	///
	/// A token created with this type will always have the value "while".
	///
	/// Like all keywords, this enum variant declaration *must* come before `Identifier`. If it doesn't, then `while` will be tokenized incorrectly as
	/// identifiers, which will cause issues when parsing. Please be careful when moving around this keyword or the `Identifier` token type!
	KeywordWhile,
	KeywordDefault,

	KeywordEither,

	/// An identifier in the language. This is essentially a "name" of a variable. Whenever the user creates a new variable with a name, it is represented with
	/// this token type.
	///
	/// Note that the token types are all checked in the order they are declared. The `Identifier` pattern **does not** take special care to not include keywords.
	/// This means that when asked, the `Identifier` token type will match keywords. This means that this variant declaration must occur *after* all keywords
	/// in the language. Please be mindful of this when moving this token type declaration around or when moving keyword token type declarations around.
	///
	/// Currently, although this may change, valid Cabin identifiers follow the following pattern:
	///
	/// ```js
	/// /[A-Za-Z_]\w*/
	/// ```
	/// without the leading and trailing slash of course. This means all identifiers must start with an alphabetical character or a underscore, and then optionally
	/// can have more characters which can be alphabetical, underscore, or a number.
	///
	/// TODO: We should consider what other symbols to allow here. Should we allow dollar signs? Why should we? Why shouldn't we? What about other unused symbols in
	/// the language like @? There should be a general design discussion about what identifiers should be valid.
	Identifier,

	LessThan,
	GreaterThan,

	/// The angle bracket parenthesis token type. This is used for function and group compile-time parameters. This will *always* come some amount of
	/// tokens before a `RightAngleBracket` token; There is no syntax in Cabin that constitutes unmatched angle brackets. This token type, when parsed, will always return
	/// a token with a single character value, which is just a left angle bracket character "<".
	LeftAngleBracket,

	/// The left brace token type. This is used for things like table creation, new scopes, etc. This should *always* come some amount of
	/// tokens before a `RightBrace` token; There is no syntax in Cabin that constitutes unmatched braces. This token type, when parsed, will always return
	/// a token with a single character value, which is just a left brace character "{".
	LeftBrace,

	/// The left bracket token type. This is used for things like list creation, list indexing, etc. This should *always* come some amount of
	/// tokens before a `RightBracket` token; There is no syntax in Cabin that constitutes unmatched brackets. This token type, when parsed, will always return
	/// a token with a single character value, which is just a left bracket character "[".
	LeftBracket,

	/// The left parenthesis token type. This is used for things like parenthesized arithmetic expressions, function calls, etc. This will *always* come some amount of
	/// tokens before a `RightParenthesis` token; There is no syntax in Cabin that constitutes unmatched parenthesis. This token type, when parsed, will always return
	/// a token with a single character value, which is just a left parenthesis character "(".
	LeftParenthesis,

	/// The "minus" token type. This is used for parsing arithmetic subtraction expressions. Any token tokenized of this type will always have a single-character value,
	/// which is the "hyphen" or "minus" value (-).
	Minus,

	/// The number token type. Currently, Cabin only supports very clear decimal literals. It does not currently support binary literals, hex literals, scientific
	/// notation literals, octal literals, etc. Furthermore, unlike Rust, Cabin does not allow number literals with a decimal but no numbers after the decimal,
	/// nor does it allow decimal literals with no numbers proceeding the decimal. To be specific, all numbers in the value currently must match the given regular
	/// expression:
	///
	/// ```js
	/// /-?\d+(\.\d+)?/
	/// ```
	///
	/// without including the leading and trailing slash of course.
	Number,

	/// The "plus" token type. This is used for parsing arithmetic addition expressions. Any token tokenized of this type will always have a single-character value,
	/// which is the "plus" value (+).
	Plus,

	/// The right angle bracket token type. This is used for function and group compile-time parameters. This will *always* come some amount of
	/// tokens after a `LeftAngleBracket` token; There is no syntax in Cabin that constitutes unmatched angle brackets. This token type, when parsed, will always return
	/// a token with a single character value, which is just a right angle bracket character ">".
	RightAngleBracket,

	/// The right brace token type. This is used for things like table creation, new scopes, etc. This will *always* come some amount of
	/// tokens after a `LeftBrace` token; There is no syntax in Cabin that constitutes unmatched brackets. This token type, when parsed, will always return
	/// a token with a single character value, which is just a right brace character "}".
	RightBrace,

	/// The right bracket token type. This is used for things like list creation, list indexing, etc. This will *always* come some amount of
	/// tokens after a `LeftBracket` token; There is no syntax in Cabin that constitutes unmatched brackets. This token type, when parsed, will always return
	/// a token with a single character value, which is just a right bracket character "]".
	RightBracket,

	/// The right parenthesis token type. This is used for things like parenthesized arithmetic expressions, function calls, etc. This will *always* come some amount of
	/// tokens after a `LeftParenthesis` token; There is no syntax in Cabin that constitutes unmatched parenthesis. This token type, when parsed, will always return
	/// a token with a single character value, which is just a right parenthesis character ")".
	RightParenthesis,

	/// The semicolon token type. These are exclusively used in the language to end statements. Cabin is not a whitespace-sensitive language, so semicolons are used to
	/// indicate the end of a statement. This token type, when parsed, will always return a token with a single character value, which is just a semicolon.
	Semicolon,

	/// The string token type. This is a double quoted string. In Cabin, all strings are formatted and multiline by default; However, the parsing of inlined formatted
	/// expressions is done at a later step, so an entire formatted string is still just returned from the lexer as a simple string. The double quotes of the string
	/// are both included in the returned token.
	String,

	/// The whitespace token type. This is a special token type because it is detected by the lexer, but tokens of this type are not added to the token list;
	/// The parser never sees them. This constitutes characters of all standard ASCII whitespace including spaces, tabs, newlines, and carriage returns (which are
	/// outside of strings of course). Cabin is not a whitespace-sensitive language, so these are intentionally ignored when tokenizing.
	Whitespace,
	Unknown,
	Unrecognized,
}

impl TokenType {
	pub fn is_whitespace(&self) -> bool {
		matches!(self, TokenType::Whitespace | TokenType::Comment)
	}

	// TODO: This could pretty easily be refactored into a non-regex solution that would almost certainly be more performant;
	// It would certainly be less clean and less concise, but it would be more performant, so we should consider this at some point.

	/// Returns a regular expression pattern that matches the token type. This specifically checks if the given string *starts* with the token type.
	/// The returned value is a lazily-evaluated static, so there is no performance loss to calling this repeatedly.
	///
	/// # Returns
	/// A regular expression pattern that matches the token type.
	fn pattern(self) -> &'static Regex {
		match self {
			// Keywords
			Self::KeywordAction => regex!(r"^action\b"),
			Self::KeywordAnd => regex!(r"^and\b"),
			Self::KeywordOr => regex!(r"^or\b"),
			Self::KeywordDefault => regex!(r"^default\b"),
			Self::KeywordEither => regex!(r"^either\b"),
			Self::KeywordExtend => regex!(r"^extend\b"),
			Self::KeywordForEach => regex!(r"^foreach\b"),
			Self::KeywordGroup => regex!(r"^group\b"),
			Self::KeywordIf => regex!(r"^if\b"),
			Self::KeywordIn => regex!(r"^in\b"),
			Self::KeywordIs => regex!(r"^is\b"),
			Self::KeywordLet => regex!(r"^let\b"),
			Self::KeywordWith => regex!(r"^with\b"),
			Self::KeywordMatch => regex!(r"^match\b"),
			Self::KeywordNew => regex!(r"^new\b"),
			Self::KeywordOneOf => regex!(r"^choice\b"),
			Self::KeywordOtherwise => regex!(r"^otherwise\b"),
			Self::KeywordRuntime => regex!(r"^run\b"),
			Self::KeywordToBe => regex!(r"^tobe\b"),
			Self::KeywordWhile => regex!(r"^while\b"),

			// Left opening groupings
			Self::LeftAngleBracket => regex!("^<"),
			Self::LeftBrace => regex!(r"^\{"),
			Self::LeftBracket => regex!(r"^\["),
			Self::LeftParenthesis => regex!(r"^\("),

			// Right closing groupings
			Self::RightAngleBracket => regex!("^>"),
			Self::RightBrace => regex!(r"^\}"),
			Self::RightBracket => regex!(r"^\]"),
			Self::RightParenthesis => regex!(r"^\)"),

			// Literals
			Self::String => regex!(r#"(?s)^"[^"]*""#),
			Self::Number => regex!(r"^-?\d+(\.\d+)?"),
			Self::Identifier => regex!(r"^[a-zA-Z_]\w*"),

			// Operators
			Self::Plus => regex!(r"^\+"),
			Self::Minus => regex!("^-"),
			Self::Asterisk => regex!(r"^\*"),
			Self::Caret => regex!(r"^\^"),
			Self::Dot => regex!(r"^\."),
			Self::DoubleEquals => regex!("^=="),
			Self::ForwardSlash => regex!("^/"),
			Self::Equal => regex!("^="),
			Self::LessThan => regex!(r"^\s+<"),
			Self::GreaterThan => regex!(r"^\s+>"),
			Self::RightArrow => regex!(r"^->"),
			Self::ExclamationPoint => regex!(r"^!"),
			Self::QuestionMark => regex!(r"^\?"),

			// Punctuations / Misc
			Self::TagOpening => regex!(r"^\#\["),
			Self::Colon => regex!("^:"),
			Self::Comma => regex!("^,"),
			Self::Semicolon => regex!("^;"),

			// Ignored tokens
			Self::Whitespace => regex!(r"^\s"),
			Self::Comment => regex!(r"^#[^\n\r#]*[\n\r#$]"),

			// Unknown - This token type only appears when using `tokenize_string`, in which case
			// it is inserted manually into the token stream. That's why this has an unmatachable
			// regex - an "end of line" indicator followed by the letter 'a' - so that it'll never
			// be naturally matched during tokenization.
			Self::Unknown => regex!(r"$a"),
			Self::Unrecognized => regex!(r"[^\s]+"),
		}
	}

	/// Returns the matched text of the token type in the given code. This only returns `Some` if there is a match *at the start* of the string to this
	/// token type. Even if this token type exists in the given code, but occurs later than the start, this will return `None`.
	///
	/// # Parameters
	/// - `code`: The code to find a match in.
	///
	/// # Returns
	/// The matched text of the token type in the given code, or `None` if no match was found.

	pub fn get_match(&self, code: &str) -> Option<String> {
		self.pattern().find(code).map(|m| m.as_str().to_owned())
	}

	/// Finds the first token type that matches the given code.
	///
	/// # Parameters
	/// - `code`: The code to find a match for.
	///
	/// # Returns
	/// The first token type that matches the given code, along with the matched text.

	fn find_match(code: &str) -> Option<(Self, String)> {
		for token_type in Self::iter() {
			if let Some(matched) = token_type.get_match(code) {
				return Some((token_type, matched));
			}
		}
		None
	}
}

impl std::fmt::Display for TokenType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", format!("{self:?}").to_case(convert_case::Case::Title))
	}
}

/// A token in source code.
#[derive(Debug)]
pub struct Token {
	/// The type of the token.
	pub token_type: TokenType,
	/// The value of the token. This value is how the token originally appears in the source code *exactly*. There are some nuances to what is considered
	/// part of the value; For example, all strings retain their quotes in this field. For information about what is considered part of the `value` for
	/// a specific token type, refer to the documentation for that specific token type.
	pub value: String,
	pub span: Span,
}

impl Token {
	fn style(&self, next: Option<&Token>) -> &Style {
		match self.token_type {
			// Comments
			TokenType::Comment => context().theme.comment(),

			// Numbers
			TokenType::Number => context().theme.number(),

			// String
			TokenType::String => context().theme.string(),

			// Keywords
			TokenType::KeywordAction
			| TokenType::KeywordNew
			| TokenType::KeywordOtherwise
			| TokenType::KeywordGroup
			| TokenType::KeywordIf
			| TokenType::KeywordRuntime
			| TokenType::KeywordIs
			| TokenType::KeywordOneOf
			| TokenType::KeywordLet
			| TokenType::KeywordForEach
			| TokenType::KeywordIn
			| TokenType::KeywordWhile
			| TokenType::KeywordDefault
			| TokenType::KeywordEither => context().theme.keyword(),

			// Identifiers
			TokenType::Identifier => {
				if let Some(next) = next {
					if next.token_type == TokenType::LeftAngleBracket || next.token_type == TokenType::LeftParenthesis {
						return context().theme.function();
					}
				}
				if self.value.starts_with(|character: char| character.is_uppercase()) {
					context().theme.type_name()
				} else {
					context().theme.variable_name()
				}
			},
			_ => context().theme.normal(),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
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

	pub fn start_line_column(&self, text: &str) -> (usize, usize) {
		let mut line = 0;
		let mut column = 0;
		for (position, character) in text.chars().enumerate() {
			if position == self.start() {
				return (line, column);
			}

			column += 1;
			if character == '\n' {
				line += 1;
				column = 0;
			}
		}

		unreachable!()
	}

	pub fn end_line_column(&self, text: &str) -> (usize, usize) {
		let mut line = 0;
		let mut column = 0;
		for (position, character) in text.chars().enumerate() {
			if position == self.end() {
				return (line, column);
			}

			column += 1;
			if character == '\n' {
				line += 1;
				column = 0;
			}
		}

		unreachable!()
	}
}

/// Tokenizes a string of Cabin source code into a vector of tokens. This is the first step in compiling Cabin source code. The returned vector of tokens
/// should be passed into the Cabin parser, which will convert it into an abstract syntax tree.
///
/// # Parameters
/// - `code` - The Cabin source code. If the given code is not valid Cabin code, this function makes no guarantees to return an error, nor does it make
/// a guarantee to return an `Ok`. This includes semantic and syntactic errors. This function will only return an error if an unrecognized token is found;
/// Meaning a piece of code is encountered that doesn't match any known token types. This could be a non-ASCII character or just generally any unused character
/// in the language like `@`.
///
/// # Returns
/// A vector of tokens in the order they appeared in the given source code after tokenization, or an `Err` if an unrecognized token was found.
///
/// # Errors
/// If the given code string is not syntactically valid Cabin code. It needn't be semantically valid, but it must be comprised of the proper tokens.
pub fn tokenize_program(code: &str, is_prelude: bool) -> VecDeque<Token> {
	let mut code = code.to_owned();

	let mut tokens = Vec::new();
	let mut position = 0;

	// We only read tokens from the start of a string, so we repeatedly loop over the code and remove the tokenized text when we find tokens.
	// This means we can just iterate while code isn't empty.
	while !code.is_empty() {
		// We've got a match - we found a token that matches the start of the code
		let Some((token_type, value)) = TokenType::find_match(&code) else { unreachable!() };

		if token_type == TokenType::Unrecognized {
			context().add_error(crate::Error {
				span: Span { start: position, length: 1 },
				error: ErrorInfo::Tokenize(TokenizeError::UnrecognizedToken(value.clone())),
			});
		}

		let length = value.len(); // This must be done early so that we aren't trying to get the length of a moved value

		// Add the token
		let token = Token {
			token_type,
			value,
			span: Span { start: position, length },
		};
		tokens.push(token);
		position += length;

		code = code.get(length..).unwrap().to_owned();
	}

	if !is_prelude {
		for (index, token) in tokens.iter().enumerate() {
			let style = token.style(tokens.get(index + 1));
			for character in token.value.chars() {
				context().colored_program.push(character.to_string().style(style));
			}
		}
	}

	VecDeque::from(tokens)
}

/// Tokenizes a string infallibly. This performs raw tokenization on a string and returns a token
/// stream as a result. Instead of erroring when encountering unknown tokens, a token of type
/// `TokenType::Unknown` is included in the token stream.
///
/// This is used by `CabinString::parse` to parse expressions in formatted strings. Formatted
/// strings contain expressions that need to be parsed (and therefore tokenized) but they will also
/// contain further tokens that may not necessarily be valid because they're part of a string
/// literal. Thus, we need an infallible tokenize function.
///
/// This shouldn't be used for tokenizing general Cabin programs; Use `tokenize` instead.
///
/// # Parameters
///
/// - `string` - The string to tokenize
///
/// # Returns
///
/// A token stream of tokenized tokens, possibly including `Unknown` tokens.
pub fn tokenize_string(string: &str) -> VecDeque<Token> {
	let mut code = string.to_owned();

	let mut tokens = Vec::new();
	let mut position = 0;

	// We only read tokens from the start of a string, so we repeatedly loop over the code and remove the tokenized text when we find tokens.
	// This means we can just iterate while code isn't empty.
	while !code.is_empty() {
		let (token_type, value) = TokenType::find_match(&code).unwrap_or((TokenType::Unknown, code.chars().next().unwrap().to_string()));
		let length = value.len(); // This must be done early so that we aren't trying to get the length of a moved value

		// Add the token
		let token = Token {
			token_type,
			value,
			span: Span { start: position, length },
		};
		tokens.push(token);
		position += length;

		code = code.get(length..).unwrap().to_owned();
	}

	VecDeque::from(tokens)
}

#[derive(Clone, thiserror::Error, Debug)]
pub enum TokenizeError {
	#[error("Unrecognized token: {0}")]
	UnrecognizedToken(String),
}

/// Tokenizes a string of Cabin source code into a vector of tokens. This is the first step in compiling Cabin source code. The returned vector of tokens
/// should be passed into the Cabin parser, which will convert it into an abstract syntax tree.
///
/// The Cabin prelude is automatically prepended to the returned token stream. To tokenize without the prelude, use `tokenize_without_prelude()`
/// (I'm very creative with names, I know).
///
/// # Parameters
///
/// - `code` - The Cabin source code. If the given code is not valid Cabin code, this function makes no guarantees to return an error, nor does it make
/// a guarantee to return an `Ok`. This includes semantic and syntactic errors. This function will only return an error if an unrecognized token is found;
/// Meaning a piece of code is encountered that doesn't match any known token types. This could be a non-ASCII character or just generally any unused character
/// in the language like `@`.
///
/// # Returns
///
/// A vector of tokens in the order they appeared in the given source code after tokenization, or an `Err` if an unrecognized token was found.
///
/// # Errors
///
/// If the given code string is not syntactically valid Cabin code. It needn't be semantically valid, but it must be comprised of the proper tokens.
pub fn tokenize(code: &str) -> Result<VecDeque<Token>, crate::Error> {
	let mut tokens = tokenize_program(code, false);
	let mut prelude_tokens = tokenize_program(PRELUDE, true);
	prelude_tokens.append(&mut tokens);
	Ok(prelude_tokens)
}

/// Tokenizes a string of Cabin source code into a vector of tokens. This is the first step in compiling Cabin source code. The returned vector of tokens
/// should be passed into the Cabin parser, which will convert it into an abstract syntax tree.
///
/// The Cabin prelude is not prepended to the returned token stream. To tokenize with the prelude, use `tokenize()`.
///
/// # Parameters
///
/// - `code` - The Cabin source code. If the given code is not valid Cabin code, this function makes no guarantees to return an error, nor does it make
/// a guarantee to return an `Ok`. This includes semantic and syntactic errors. This function will only return an error if an unrecognized token is found;
/// Meaning a piece of code is encountered that doesn't match any known token types. This could be a non-ASCII character or just generally any unused character
/// in the language like `@`.
///
/// # Returns
///
/// A vector of tokens in the order they appeared in the given source code after tokenization, or an `Err` if an unrecognized token was found.
///
/// # Errors
///
/// If the given code string is not syntactically valid Cabin code. It needn't be semantically valid, but it must be comprised of the proper tokens.
pub fn tokenize_without_prelude(code: &str) -> VecDeque<Token> {
	tokenize_program(code, false)
}

/// Tokenizes the main entry point for a Cabin program. This is identical to `tokenize`, but it wraps the code in a function declaration.
///
/// # Parameters
///
/// - `code` - The source code to tokenize
///
/// # Returns
///
/// The token stream that was generated from the source code.
///
/// # Errors
///
/// If an unrecognized token was encountered.
pub fn tokenize_main(code: &str) -> anyhow::Result<VecDeque<Token>> {
	let mut tokens = tokenize(code)?;
	let mut pre_tokens = tokenize_program("let main_function = action {", true);
	let mut post_tokens = tokenize_program("};", true);
	pre_tokens.append(&mut tokens);
	pre_tokens.append(&mut post_tokens);
	Ok(pre_tokens)
}
