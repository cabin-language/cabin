use std::collections::HashMap;

use colored::{ColoredString, Colorize as _};

pub trait Theme {
	fn keyword() -> (u8, u8, u8);
	fn string() -> (u8, u8, u8);
	fn parameter() -> (u8, u8, u8);
	fn type_() -> (u8, u8, u8);
	fn function_call() -> (u8, u8, u8);
	fn number() -> (u8, u8, u8);
	fn field() -> (u8, u8, u8);
	fn comment() -> (u8, u8, u8);
	fn normal() -> (u8, u8, u8);
	fn background() -> (u8, u8, u8);
	fn special_punctuation() -> (u8, u8, u8);
	fn grouping_punctuation() -> (u8, u8, u8);
	fn error() -> (u8, u8, u8);
	fn warning() -> (u8, u8, u8);
	fn error_background() -> (u8, u8, u8);
	fn warning_background() -> (u8, u8, u8);

	fn highlight(query: &str) -> Option<(u8, u8, u8)> {
		let highlight = HashMap::from([
			("function.call", Self::function_call()),
			("variable.parameter", Self::parameter()),
			("keyword", Self::keyword()),
			("keyword.function", Self::keyword()),
			("type", Self::type_()),
			("number", Self::number()),
			("variable.member", Self::field()),
			("punctuation.special", Self::special_punctuation()),
			("punctuation.bracket", Self::grouping_punctuation()),
			("comment", Self::comment()),
			("string", Self::string()),
		]);
		highlight.get(query).copied()
	}
}

pub struct CatppuccinMocha;

impl Theme for CatppuccinMocha {
	fn normal() -> (u8, u8, u8) {
		(205, 214, 244)
	}

	fn special_punctuation() -> (u8, u8, u8) {
		(245, 194, 231)
	}

	fn grouping_punctuation() -> (u8, u8, u8) {
		(147, 153, 178)
	}

	fn string() -> (u8, u8, u8) {
		(166, 227, 161)
	}

	fn parameter() -> (u8, u8, u8) {
		(243, 139, 168)
	}

	fn type_() -> (u8, u8, u8) {
		(249, 226, 175)
	}

	fn keyword() -> (u8, u8, u8) {
		(203, 166, 247)
	}

	fn comment() -> (u8, u8, u8) {
		(147, 153, 178)
	}

	fn function_call() -> (u8, u8, u8) {
		(137, 180, 250)
	}

	fn number() -> (u8, u8, u8) {
		(250, 179, 135)
	}

	fn field() -> (u8, u8, u8) {
		(180, 190, 254)
	}

	fn background() -> (u8, u8, u8) {
		(30, 30, 46)
	}

	fn error() -> (u8, u8, u8) {
		(243, 139, 168)
	}

	fn error_background() -> (u8, u8, u8) {
		(50, 40, 58)
	}

	fn warning() -> (u8, u8, u8) {
		(249, 226, 175)
	}

	fn warning_background() -> (u8, u8, u8) {
		(51, 49, 48)
	}
}

pub trait Themed {
	fn error<T: Theme>(&self) -> ColoredString;
	fn highlighted_error<T: Theme>(&self) -> ColoredString;
}

impl<S: AsRef<str>> Themed for S {
	fn error<T: Theme>(&self) -> ColoredString {
		let (r, g, b) = T::error();
		self.as_ref().truecolor(r, g, b)
	}

	fn highlighted_error<T: Theme>(&self) -> ColoredString {
		let (fg_r, fg_g, fg_b) = T::error();
		let (bg_r, bg_g, bg_b) = T::error_background();
		self.as_ref().truecolor(fg_r, fg_g, fg_b).on_truecolor(bg_r, bg_g, bg_b)
	}
}
