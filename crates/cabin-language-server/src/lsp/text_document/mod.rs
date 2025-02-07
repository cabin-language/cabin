pub mod diagnostics;
pub mod did_change;
pub mod hover;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentItem {
	pub uri: String,
	pub language_id: String,
	pub version: u32,
	pub text: String,
}

impl TextDocumentItem {
	pub fn did_open(&self) {}
}

#[derive(serde::Deserialize)]
pub struct VersionTextDocumentIdentifier {
	pub uri: String,
	pub version: u32,
}

#[derive(serde::Deserialize)]
pub struct TextDocumentIdentifier {
	pub uri: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Position {
	line: usize,
	character: usize,
}

impl Position {
	pub fn to_span(&self, text: &str) -> cabin::lexer::Span {
		let mut line = 0;
		let mut column = 0;
		for (position, character) in text.chars().enumerate() {
			if line as usize == self.line && column as usize == self.character {
				return cabin::lexer::Span::new(position as usize, 1);
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

impl From<(usize, usize)> for Position {
	fn from(value: (usize, usize)) -> Self {
		Self {
			line: value.0,
			character: value.1,
		}
	}
}

#[derive(serde::Serialize)]
pub struct Range {
	pub start: Position,
	pub end: Position,
}
