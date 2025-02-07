use super::{Position, TextDocumentIdentifier};

#[derive(serde::Deserialize)]
pub struct TextDocumentPositionParams {
	position: Position,
	text_document: TextDocumentIdentifier,
}

#[derive(serde::Serialize)]
pub struct HoverResult {
	pub contents: String,
}
