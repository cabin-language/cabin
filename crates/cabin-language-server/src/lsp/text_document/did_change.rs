#[derive(serde::Deserialize)]
pub struct TextDocumentDidChangeEvent {
	pub text: String,
}
