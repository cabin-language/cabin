use cabin::{parser::expressions::Spanned as _, DiagnosticInfo};

use super::Range;
use crate::{lsp::State, Logger};

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

fn diagnostic_code(diagnostic: &cabin::Diagnostic) -> u8 {
	match diagnostic.info() {
		DiagnosticInfo::Error(_) => 1,
		DiagnosticInfo::Warning(_) => 2,
	}
}

pub fn get_diagnostics(state: &State, logger: &mut Logger, uri: &str) -> anyhow::Result<Vec<Diagnostic>> {
	let code = state.files.get(uri).unwrap();
	logger.log("\n*Checking for diagnostics...*")?;
	let diagnostics = cabin::check(code);
	logger.log("\n*Done checking. Reporting diagnostics.*")?;
	Ok(diagnostics
		.into_iter()
		.map(|diagnostic| Diagnostic {
			severity: diagnostic_code(&diagnostic),
			message: format!("{diagnostic}"),
			source: "Cabin Language Server".to_owned(),
			range: Range {
				start: diagnostic.span().start_line_column(code).into(),
				end: diagnostic.span().end_line_column(code).into(),
			},
		})
		.collect::<Vec<_>>())
}
