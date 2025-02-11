use cabin::{diagnostics::DiagnosticInfo, Spanned as _};

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

fn diagnostic_code(diagnostic: &cabin::diagnostics::Diagnostic) -> u8 {
	match diagnostic.info() {
		DiagnosticInfo::Error(_) => 1,
		DiagnosticInfo::Warning(_) => 2,
	}
}

pub fn get_diagnostics(state: &State, logger: &mut Logger, uri: &str) -> anyhow::Result<Vec<Diagnostic>> {
	let code = state.files.get(uri).unwrap();
	logger.log("\n*Checking for diagnostics...*")?;
	let context = if uri.ends_with("/main.cabin") {
		cabin::check_program(code)
	} else {
		cabin::check_module(code)
	};
	logger.log("\n*Done checking. Reporting diagnostics.*")?;
	Ok(context
		.diagnostics()
		.to_owned()
		.into_iter()
		.map(|diagnostic| {
			let span = diagnostic.span(&context);
			Diagnostic {
				severity: diagnostic_code(&diagnostic),
				message: format!("{diagnostic}"),
				source: "Cabin Language Server".to_owned(),
				range: Range {
					start: span.start_line_column(code).unwrap_or((0, 0)).into(),
					end: diagnostic.span(&context).end_line_column(code).unwrap_or((0, 0)).into(),
				},
			}
		})
		.collect::<Vec<_>>())
}
