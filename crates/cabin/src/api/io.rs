use std::io::{Stderr, Stdin, Stdout, Write};

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

pub trait IoReader {
	fn read(&mut self) -> String;
}

pub trait IoWriter {
	fn write(&mut self, value: &StyledString);

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
}

impl IoWriter for Stdout {
	fn write(&mut self, value: &StyledString) {
		let colored: ColoredString = value.into();
		print!("{colored}");
		self.flush().unwrap();
	}
}

impl IoWriter for Stderr {
	fn write(&mut self, value: &StyledString) {
		let colored: ColoredString = value.into();
		eprint!("{colored}");
		self.flush().unwrap();
	}
}

impl IoReader for Stdin {
	fn read(&mut self) -> String {
		let mut line = String::new();
		let _ = std::io::stdin().read_line(&mut line).unwrap();
		line = line.get(0..line.len() - 1).unwrap().to_owned();
		line
	}
}
pub struct Io<Input: IoReader, Output: IoWriter, Error: IoWriter> {
	pub input: Input,
	pub output: Output,
	pub error: Error,
}
