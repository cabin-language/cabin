use super::Range;

#[derive(serde::Serialize)]
pub struct PublishDiagnosticParams {
	pub uri: String,
	pub diagnostics: Vec<Diagnostic>,
}

#[derive(serde::Serialize)]
pub struct Diagnostic {
	pub range: Range,
	pub severity: u8,
	pub source: String,
	pub message: String,
}
