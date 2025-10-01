use std::io::Write as _;

use colored::{ColoredString, Colorize as _};
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Copy)]
#[wasm_bindgen]
pub struct Color {
	red: u8,
	green: u8,
	blue: u8,
}

#[derive(Clone)]
#[wasm_bindgen]
pub struct StyledString {
	#[wasm_bindgen(skip)]
	pub value: String,
	pub color: Option<Color>,
	pub bold: bool,
	pub italic: bool,
	pub underline: bool,
	pub undercurl: bool,
}

#[wasm_bindgen]
impl StyledString {
	#[wasm_bindgen(getter)]
	pub fn value(&self) -> String {
		self.value.clone()
	}
}

#[allow(clippy::multiple_inherent_impl, reason = "these must be separate because one is #[wasm_bindgen] and one is not")]
impl StyledString {
	pub fn plain<S: AsRef<str>>(value: S) -> Self {
		StyledString {
			value: value.as_ref().to_owned(),
			color: None,
			bold: false,
			italic: false,
			underline: false,
			undercurl: false,
		}
	}
}

impl From<&StyledString> for ColoredString {
	fn from(value: &StyledString) -> Self {
		let mut result = value.value.normal();

		if let Some(Color { red, green, blue }) = value.color {
			result = result.truecolor(red, green, blue);
		}

		if value.bold {
			result = result.bold();
		}

		if value.italic {
			result = result.italic();
		}

		if value.underline {
			result = result.underline();
		}

		result
	}
}

/// A generic system input-output interaction interface. The entire Cabin compiler is generic over
/// this interface, meaning that arbitrary virtual systems can be used in conjunction with the
/// compiler.
///
/// The `Io` trait provides methods for interacting with a "system". on the system-level. It
/// includes functions for reading input, writing output, interacting with files and environment
/// variables, etc.
///
/// For the standard `SystemIo`, these are implemented with their standard values: Reading from
/// `stdin`, writing to `stdout` and `stderr`, interacting with files and environment variables
/// straight from the system.
///
/// Other systems can manually implement `Io` to specify how to interact with them. For example, on
/// the Cabin website, there's a playground that allows running arbitrary Cabin code. This uses a
/// "virtual system" that keeps track of environment variables and files on the front-end. The
/// front-end provides an implementation of this trait, and handles system interaction accordingly.
pub trait Io {
	fn read_line(&mut self) -> String;
	fn write(&mut self, value: &StyledString);
	fn error_write(&mut self, value: &StyledString);

	fn get_environment_variable(&mut self, name: &str) -> Option<String>;
	fn set_environment_variable(&mut self, name: &str, value: &str);

	fn read_file(&mut self, path: &str) -> Option<String>;
	fn write_file(&mut self, path: &str, contents: &str);
	fn delete_file(&mut self, path: &str);

	fn append_file(&mut self, path: &str, contents: &str) {
		let contents = self.read_file(path).unwrap() + contents;
		self.write_file(path, &contents);
	}

	fn writeln(&mut self, value: &StyledString) {
		self.write(value);
		self.write(&StyledString::plain("\n"));
	}

	fn write_all(&mut self, values: &[StyledString]) {
		for value in values {
			self.write(value);
		}
	}

	fn write_all_line(&mut self, values: &[StyledString]) {
		for value in values {
			self.write(value);
		}
		self.write(&StyledString::plain("\n"));
	}

	fn write_lines(&mut self, values: &[StyledString]) {
		for value in values {
			self.writeln(value);
		}
	}

	fn error_writeln(&mut self, value: &StyledString) {
		self.write(value);
		self.write(&StyledString::plain("\n"));
	}

	fn error_write_all(&mut self, values: &[StyledString]) {
		for value in values {
			self.write(value);
		}
	}

	fn error_write_all_line(&mut self, values: &[StyledString]) {
		for value in values {
			self.write(value);
		}
		self.write(&StyledString::plain("\n"));
	}

	fn error_write_lines(&mut self, values: &[StyledString]) {
		for value in values {
			self.writeln(value);
		}
	}

	fn file_exists(&mut self, path: &str) -> bool {
		self.read_file(path).is_some()
	}
}

/// A standard system implementation of [`Io`]. This uses the standard input, output, and error
/// streams for reading and writing, and interacts with the system's literal file system,
/// environment variables, etc.
///
/// When creating a [`StandardContext`](crate::context::StandardContext), this is the `Io`
/// implementation used.
pub struct SystemIo;

impl Io for SystemIo {
	fn read_line(&mut self) -> String {
		let mut line = String::new();
		let _ = std::io::stdin().read_line(&mut line).unwrap();
		line
	}

	fn write(&mut self, value: &StyledString) {
		let colored: ColoredString = value.into();
		print!("{colored}");
		std::io::stdout().flush().unwrap();
	}

	fn error_write(&mut self, value: &StyledString) {
		let colored: ColoredString = value.into();
		eprint!("{colored}");
		std::io::stdout().flush().unwrap();
	}

	fn get_environment_variable(&mut self, name: &str) -> Option<String> {
		std::env::var(name).ok()
	}

	fn set_environment_variable(&mut self, name: &str, value: &str) {
		std::env::set_var(name, value);
	}

	fn read_file(&mut self, path: &str) -> Option<String> {
		std::fs::read_to_string(path).ok()
	}

	fn write_file(&mut self, path: &str, contents: &str) {
		std::fs::write(path, contents).unwrap();
	}

	fn delete_file(&mut self, path: &str) {
		std::fs::remove_file(path).unwrap();
	}
}
